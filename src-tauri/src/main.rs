#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod configuration;
mod diagnostics;
mod discovery;
mod issue;
mod labels;
mod language;
mod process;
mod profile;
mod sasuser;
mod session;
mod startup;
mod store;
mod theme;
mod view;

use discovery::{BuiltIn, Installation};
use issue::Issue;
use language::LanguagePreference;
use profile::Layers;
use serde::{Deserialize, Serialize};
use session::{FailureRequest, LaunchRequest, Purpose, Record, Sessions, Stage, Status};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    time::Duration,
};
use store::{ConfigBase, ConfigRef, ProfileOrigin, Project, ProjectProfile, Store};
use tauri::{
    AppHandle, Emitter, Manager, State, WindowEvent,
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
};
use theme::ThemePreference;
use uuid::Uuid;
use view::{SessionView, View};

/// A project whose SAS session is still running must not be launched twice: the
/// two processes would share one sasuser folder and one set of logs.
fn project_running(records: &[Record], id: Uuid) -> bool {
    records.iter().any(|record| {
        record.status == Status::Running && record.purpose == Purpose::Project { id: Some(id) }
    })
}

struct App {
    pub(crate) store: Store,
    pub(crate) sessions: Sessions,
    pub(crate) installation: Option<Installation>,
    pub(crate) scan_error: Option<Issue>,
    pub(crate) system_language: String,
    /// Registry truth for the login entry, refreshed whenever it changes.
    pub(crate) autostart: bool,
}

type AppState = Mutex<Result<App, Issue>>;

#[derive(Debug)]
struct Resolved {
    cfg: PathBuf,
    encoding: Option<String>,
    locale: Option<String>,
    factory: PathBuf,
    thin: Option<PathBuf>,
    profile: Option<PathBuf>,
    /// A project's own autoexec file; only `resolved_for` ever fills it in.
    startup: Option<PathBuf>,
    /// Whether the launch tells the session where that file is.
    startup_exposes_path: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchResult {
    id: Uuid,
    name: Option<String>,
    status: &'static str,
    reason: Option<Issue>,
}

impl App {
    fn new(root: PathBuf) -> Result<Self, Issue> {
        Self::open(Store::open(root)?)
    }

    /// Build the app without probing the machine, so tests stay deterministic.
    fn open(store: Store) -> Result<Self, Issue> {
        let mut app = Self {
            sessions: Sessions::new(store.sessions_dir()),
            store,
            installation: None,
            scan_error: None,
            system_language: language::system_language(),
            autostart: false,
        };
        app.refresh();
        Ok(app)
    }

    fn exe(&self) -> Option<PathBuf> {
        self.store
            .settings
            .exe
            .clone()
            .or_else(discovery::find_installed)
    }

    fn refresh(&mut self) {
        let result = self
            .exe()
            .ok_or(Issue::new("sasMissing"))
            .and_then(|exe| discovery::inspect(&exe));
        match result {
            Ok(installation) => {
                self.installation = Some(installation);
                self.scan_error = None;
            }
            Err(error) => {
                self.installation = None;
                self.scan_error = Some(error);
            }
        }
    }

    fn factory_configs(&self) -> &[BuiltIn] {
        self.installation
            .as_ref()
            .map(|installation| installation.configs.as_slice())
            .unwrap_or(&[])
    }

    fn missing_installation(&self) -> Issue {
        self.scan_error
            .clone()
            .unwrap_or_else(|| Issue::new("sasMissing"))
    }

    /// Update statuses and persist launch outcomes of the sessions. Called by
    /// the 3-second background thread and by every command that reads state.
    fn maintenance(&mut self) {
        // A project snapshot is committed by the session layer, so the cached
        // project metadata has to be re-read before the next launch uses it.
        if self.sessions.sweep() {
            let _ = self.store.reload();
        }
        let failures = self.sessions.take_failures();
        if failures.is_empty() {
            return;
        }
        let factory: Vec<BuiltIn> = self.factory_configs().to_vec();
        let _ = self.store.update(|settings| {
            for (encoding, locale) in &failures {
                if !configuration::is_factory_combo(&factory, encoding, locale) {
                    configuration::record_launch(&mut settings.presets, encoding, locale, false);
                }
            }
        });
    }

    fn record_preset(&mut self, encoding: &str, locale: &str, ok: bool) {
        if configuration::is_factory_combo(self.factory_configs(), encoding, locale) {
            return;
        }
        let encoding = encoding.to_owned();
        let locale = locale.to_owned();
        let _ = self.store.update(|settings| {
            configuration::record_launch(&mut settings.presets, &encoding, &locale, ok)
        });
    }

    fn view(&mut self) -> View {
        self.maintenance();
        view::build_view(self)
    }

    fn session_view(&self, record: &Record) -> SessionView {
        view::build_session_view(self, record)
    }

    fn resolve(&self, reference: Option<&ConfigRef>) -> Result<Resolved, Issue> {
        let installation = self
            .installation
            .as_ref()
            .ok_or_else(|| self.missing_installation())?;
        match reference {
            None => {
                let cfg = discovery::resolved_default_cfg(&installation.exe)?;
                let code = cfg
                    .parent()
                    .and_then(Path::file_name)
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                Self::resolved_from(discovery::describe(&cfg, code), cfg, None, None)
            }
            Some(ConfigRef::BuiltIn { code }) => {
                let cfg = discovery::factory_cfg_for(&installation.exe, code)?;
                Self::resolved_from(discovery::describe(&cfg, code.clone()), cfg, None, None)
            }
            Some(ConfigRef::User { id }) => {
                let config = self
                    .store
                    .configuration(*id)
                    .ok_or_else(|| Issue::new("configUnavailable"))?;
                let thin = self.store.thin_cfg_path(config);
                if !is_regular_file(&thin) {
                    return Err(Issue::new("configurationFileMissing"));
                }
                if !is_regular_file(&config.factory) {
                    return Err(Issue::new("configurationBaseMissing"));
                }
                Ok(Resolved {
                    cfg: thin.clone(),
                    encoding: Some(config.encoding.clone()),
                    locale: Some(config.locale.clone()),
                    factory: config.factory.clone(),
                    thin: Some(thin),
                    profile: self
                        .store
                        .profile_revision_dir(config)
                        .filter(|directory| profile::has_catalog_files(directory)),
                    startup: None,
                    startup_exposes_path: false,
                })
            }
        }
    }

    fn resolved_from(
        built: BuiltIn,
        cfg: PathBuf,
        thin: Option<PathBuf>,
        profile: Option<PathBuf>,
    ) -> Result<Resolved, Issue> {
        if let Some(error) = built.error {
            return Err(error);
        }
        Ok(Resolved {
            cfg,
            encoding: built.encoding,
            locale: built.locale,
            factory: built.cfg,
            thin,
            profile,
            startup: None,
            startup_exposes_path: false,
        })
    }

    fn launch(
        &mut self,
        resolved: Resolved,
        name: String,
        purpose: Purpose,
    ) -> Result<Record, Issue> {
        let exe = self
            .installation
            .as_ref()
            .map(|installation| installation.exe.clone())
            .ok_or_else(|| self.missing_installation())?;
        let encoding = resolved
            .encoding
            .clone()
            .ok_or_else(|| Issue::new("encodingUnknown"))?;
        let result = self.sessions.launch(LaunchRequest {
            purpose,
            name,
            cfg: resolved.cfg,
            thin_cfg: resolved.thin,
            factory_cfg: resolved.factory,
            encoding: encoding.clone(),
            locale: resolved.locale.clone(),
            profile: resolved.profile,
            startup: resolved.startup,
            expose_startup_path: resolved.startup_exposes_path,
            exe,
        });
        if let Some(locale) = resolved.locale.as_deref() {
            self.record_preset(&encoding, locale, result.is_ok());
        }
        result
    }

    /// Factory cfg and options for a new user configuration.
    fn base_definition(
        &self,
        base: &ConfigBase,
    ) -> Result<(PathBuf, String, String, Option<PathBuf>), Issue> {
        match base {
            ConfigBase::Default => {
                let installation = self
                    .installation
                    .as_ref()
                    .ok_or_else(|| self.missing_installation())?;
                let cfg = discovery::resolved_default_cfg(&installation.exe)?;
                let built = discovery::describe(&cfg, String::new());
                Self::configuration_options(built, cfg, None)
            }
            ConfigBase::BuiltIn { code } => {
                let installation = self
                    .installation
                    .as_ref()
                    .ok_or_else(|| self.missing_installation())?;
                let cfg = discovery::factory_cfg_for(&installation.exe, code)?;
                let built = discovery::describe(&cfg, code.clone());
                Self::configuration_options(built, cfg, None)
            }
            ConfigBase::User { id } => {
                let config = self
                    .store
                    .configuration(*id)
                    .ok_or_else(|| Issue::new("configUnavailable"))?;
                if !is_regular_file(&config.factory) {
                    return Err(Issue::new("configurationBaseMissing"));
                }
                Ok((
                    config.factory.clone(),
                    config.encoding.clone(),
                    config.locale.clone(),
                    self.store
                        .profile_revision_dir(config)
                        .filter(|directory| profile::has_catalog_files(directory)),
                ))
            }
        }
    }

    fn configuration_options(
        built: BuiltIn,
        cfg: PathBuf,
        profile: Option<PathBuf>,
    ) -> Result<(PathBuf, String, String, Option<PathBuf>), Issue> {
        if let Some(error) = built.error {
            return Err(error);
        }
        let encoding = built
            .encoding
            .ok_or_else(|| Issue::new("encodingUnknown"))?;
        let locale = built.locale.unwrap_or_default();
        configuration::validate_options(&encoding, &locale)?;
        Ok((cfg, encoding, locale, profile))
    }

    fn base_label(&self, base: &ConfigBase) -> String {
        let language = self.store.settings.language.resolve(&self.system_language);
        match base {
            ConfigBase::Default => configuration::default_base_label(language),
            ConfigBase::BuiltIn { code } => code.clone(),
            ConfigBase::User { id } => self
                .store
                .configuration(*id)
                .map(|config| config.name.clone())
                .unwrap_or_else(|| configuration::default_base_label(language)),
        }
    }

    fn create_configuration(
        &mut self,
        base: &ConfigBase,
        name: Option<String>,
    ) -> Result<Uuid, Issue> {
        self.refresh();
        let (factory, encoding, locale, profile) = self.base_definition(base)?;
        let name = match name {
            Some(name) => validate_configuration_name(&name)?,
            None => configuration::default_config_name(
                self.store.settings.language.resolve(&self.system_language),
                &self.base_label(base),
                self.store.configurations(),
            ),
        };
        self.store.create_configuration(store::CreateConfiguration {
            name,
            base: base.clone(),
            factory,
            encoding,
            locale,
            profile,
        })
    }

    fn update_configuration(
        &mut self,
        id: Uuid,
        encoding: String,
        locale: String,
    ) -> Result<(), Issue> {
        configuration::validate_options(&encoding, &locale)?;
        self.refresh();
        let config = self
            .store
            .configuration(id)
            .cloned()
            .ok_or_else(|| Issue::new("managedConfigurationMissing"))?;
        self.maintenance();
        if self
            .sessions
            .setup_session(id)
            .is_some_and(|record| matches!(record.status, Status::Running | Status::Unknown))
        {
            return Err(Issue::new("configurationSetupActive"));
        }
        if !is_regular_file(&config.factory) {
            return Err(Issue::new("configurationBaseMissing"));
        }
        self.store
            .update_configuration(id, encoding.trim(), locale.trim())
    }

    fn delete_configuration(&mut self, id: Uuid) -> Result<(), Issue> {
        self.refresh();
        self.store
            .configuration(id)
            .cloned()
            .ok_or_else(|| Issue::new("managedConfigurationMissing"))?;
        self.maintenance();
        let count = self
            .store
            .projects()
            .iter()
            .filter(|project| project.config == Some(ConfigRef::User { id }))
            .count();
        if count > 0 {
            return Err(Issue::with_detail("configurationInUse", count));
        }
        if self.sessions.setup_session(id).is_some() {
            return Err(Issue::new("configurationSetupActive"));
        }
        self.store.delete_configuration(id)
    }

    fn start_config_setup(&mut self, id: Uuid) -> Result<(), Issue> {
        self.refresh();
        let config = self
            .store
            .configuration(id)
            .cloned()
            .ok_or_else(|| Issue::new("managedConfigurationMissing"))?;
        self.maintenance();
        if self.sessions.setup_session(id).is_some() {
            return Ok(());
        }
        let thin = self.store.thin_cfg_path(&config);
        if !is_regular_file(&thin) {
            return Err(Issue::new("configurationFileMissing"));
        }
        if !is_regular_file(&config.factory) {
            return Err(Issue::new("configurationBaseMissing"));
        }
        let resolved = Resolved {
            cfg: thin.clone(),
            encoding: Some(config.encoding.clone()),
            locale: Some(config.locale.clone()),
            factory: config.factory.clone(),
            thin: Some(thin),
            profile: self
                .store
                .profile_revision_dir(&config)
                .filter(|directory| profile::has_catalog_files(directory)),
            // A settings session builds a clean snapshot: the project's own
            // autoexec file must never run here.
            startup: None,
            startup_exposes_path: false,
        };
        self.launch(resolved, config.name.clone(), Purpose::Setup { config: id })?;
        Ok(())
    }

    fn finish_config_setup(&mut self, id: Uuid, save: bool) -> Result<(), Issue> {
        self.sessions.finish_setup(id, save)?;
        // The session layer commits profile_revision directly; reload metadata.
        self.store.reload()
    }

    /// Resolve a project's launch config and let its own preference choice own
    /// the seed: either its snapshot or nothing at all. Only projects that
    /// predate the field keep following their configuration. The single and the
    /// batch launch both go through here so they cannot drift apart.
    fn resolved_for(&self, project: &Project) -> Result<Resolved, Issue> {
        let mut resolved = self.resolve(project.config.as_ref())?;
        if project.profile.is_some() {
            resolved.profile = self.store.project_profile_source(project);
        }
        resolved.startup = validate_startup_file(project.startup_file.clone())?;
        ensure_startup_path_available(resolved.startup.as_deref(), project.startup_exposes_path)?;
        resolved.startup_exposes_path = project.startup_exposes_path;
        Ok(resolved)
    }

    fn launch_project(&mut self, id: Uuid) -> Result<Record, Issue> {
        self.refresh();
        let project = self
            .store
            .projects()
            .iter()
            .find(|project| project.id == id)
            .cloned()
            .ok_or_else(|| Issue::new("projectMissing"))?;
        if project_running(&self.sessions.records(), project.id) {
            return Err(Issue::new("projectRunning"));
        }
        let resolved = match self.resolved_for(&project) {
            Ok(resolved) => resolved,
            Err(error) => {
                let error = self.record_launch_failure(
                    Purpose::Project {
                        id: Some(project.id),
                    },
                    project.name.clone(),
                    project.config.as_ref(),
                    error,
                );
                return Err(error);
            }
        };
        self.launch(
            resolved,
            project.name.clone(),
            Purpose::Project {
                id: Some(project.id),
            },
        )
    }

    /// Persist a pre-spawn failure and hand the issue back so the caller can
    /// surface it, while the diagnostics page still gets a record.
    fn record_launch_failure(
        &mut self,
        purpose: Purpose,
        name: String,
        config: Option<&ConfigRef>,
        error: Issue,
    ) -> Issue {
        let (thin_cfg, factory_cfg, encoding, locale) = self.failure_chain(config);
        let stage = failure_stage(error.code);
        let reported = error.clone();
        self.sessions.record_failure(FailureRequest {
            purpose,
            name,
            stage,
            failure: error,
            factory_cfg,
            thin_cfg,
            encoding,
            locale,
        });
        reported
    }

    /// Best-effort paths and options for a configuration that failed to resolve.
    fn failure_chain(
        &self,
        config: Option<&ConfigRef>,
    ) -> (
        Option<PathBuf>,
        Option<PathBuf>,
        Option<String>,
        Option<String>,
    ) {
        match config {
            Some(ConfigRef::User { id }) => match self.store.configuration(*id) {
                Some(config) => (
                    Some(self.store.thin_cfg_path(config)),
                    Some(config.factory.clone()),
                    Some(config.encoding.clone()),
                    Some(config.locale.clone()),
                ),
                None => (None, None, None, None),
            },
            Some(ConfigRef::BuiltIn { code }) => {
                let factory = self.installation.as_ref().and_then(|installation| {
                    installation
                        .configs
                        .iter()
                        .find(|config| config.code == *code)
                        .map(|config| config.cfg.clone())
                });
                (None, factory, None, None)
            }
            None => {
                let factory = self.installation.as_ref().and_then(|installation| {
                    installation
                        .configs
                        .iter()
                        .find(|config| {
                            installation.default.as_deref() == Some(config.code.as_str())
                        })
                        .map(|config| config.cfg.clone())
                });
                (None, factory, None, None)
            }
        }
    }

    fn launch_batch(&mut self, ids: Vec<Uuid>) -> Vec<BatchResult> {
        self.refresh();
        self.maintenance();
        let mut results = Vec::new();
        let mut selected: HashSet<Uuid> = ids.iter().copied().collect();
        let projects = self.store.projects().to_vec();
        let records = self.sessions.records();
        for project in &projects {
            if !selected.remove(&project.id) {
                continue;
            }
            if project_running(&records, project.id) {
                results.push(BatchResult {
                    id: project.id,
                    name: Some(project.name.clone()),
                    status: "skipped",
                    reason: Some(Issue::new("batchRunning")),
                });
                continue;
            }
            let purpose = Purpose::Project {
                id: Some(project.id),
            };
            let outcome = match self.resolved_for(project) {
                Ok(resolved) => self.launch(resolved, project.name.clone(), purpose.clone()),
                Err(error) => Err(self.record_launch_failure(
                    purpose.clone(),
                    project.name.clone(),
                    project.config.as_ref(),
                    error,
                )),
            };
            let (status, reason) = match outcome {
                Ok(_) => ("started", None),
                Err(reason) => ("failed", Some(reason)),
            };
            results.push(BatchResult {
                id: project.id,
                name: Some(project.name.clone()),
                status,
                reason,
            });
        }
        for id in ids {
            if selected.remove(&id) {
                results.push(BatchResult {
                    id,
                    name: None,
                    status: "failed",
                    reason: Some(Issue::new("projectMissing")),
                });
            }
        }
        results
    }

    fn clear_diagnostics(&mut self) -> Result<(), Issue> {
        self.sessions.clear()
    }

    fn session_path(&mut self, id: Uuid, sasuser: bool) -> Result<PathBuf, Issue> {
        self.maintenance();
        let record = self
            .sessions
            .record(id)
            .cloned()
            .ok_or_else(|| Issue::new("pathUnavailable"))?;
        Ok(if sasuser {
            record.sasuser
        } else {
            self.sessions.log_path(record.id)
        })
    }
}

fn is_regular_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file())
}

fn open_path(path: &Path) -> Result<(), Issue> {
    if !path.exists() {
        return Err(Issue::with_detail(
            "pathUnavailable",
            path.display().to_string(),
        ));
    }
    Command::new("explorer.exe")
        .arg(path)
        .spawn()
        .map_err(|error| Issue::with_detail("spawnFailed", error))?;
    Ok(())
}

/// The launcher's own pages, so no command ever takes a URL from the renderer.
const HOME_PAGE: &str = "https://github.com/wtwang1998/sas-launcher";
const ISSUES_PAGE: &str = "https://github.com/wtwang1998/sas-launcher/issues/new";

fn open_url(url: &str) -> Result<(), Issue> {
    Command::new("explorer.exe")
        .arg(url)
        .spawn()
        .map_err(|error| Issue::with_detail("openLinkFailed", error))?;
    Ok(())
}

/// A release page for one tag. Only the tag reaches the URL, so it is validated
/// here: a renderer that could pass a path could pass a whole URL.
fn release_url(tag: &str) -> Result<String, Issue> {
    let valid = !tag.is_empty()
        && tag.len() <= 32
        && tag
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
    if !valid {
        return Err(Issue::with_detail("releaseTagInvalid", tag));
    }
    Ok(format!("{HOME_PAGE}/releases/tag/{tag}"))
}

fn with_app<T>(
    state: &AppState,
    action: impl FnOnce(&mut App) -> Result<T, Issue>,
) -> Result<T, Issue> {
    let mut app = lock_app(state);
    action(app.as_mut().map_err(|error| error.clone())?)
}

/// A command that panicked poisons the shared mutex. Everything it guards is a
/// cache over files written atomically, so recovering keeps the app usable
/// instead of failing every later command; the panic still reaches stderr.
fn lock_app(state: &AppState) -> std::sync::MutexGuard<'_, Result<App, Issue>> {
    state.lock().unwrap_or_else(|poisoned| {
        eprintln!("recovering the app state after a panic in a command");
        poisoned.into_inner()
    })
}

fn validate_name(name: &str) -> Result<String, Issue> {
    let name = name.trim();
    if name.is_empty() || name.chars().any(|ch| ch.is_control()) || name.chars().count() > 100 {
        return Err(Issue::new("invalidName"));
    }
    Ok(name.into())
}

fn validate_configuration_name(name: &str) -> Result<String, Issue> {
    let name = name.trim();
    if name.is_empty() || name.chars().any(char::is_control) || name.chars().count() > 100 {
        return Err(Issue::new("invalidConfigurationName"));
    }
    Ok(name.into())
}

fn validate_note(note: Option<String>) -> Result<Option<String>, Issue> {
    let Some(note) = note else { return Ok(None) };
    let note = note.trim();
    if note.chars().count() > 2000
        || note
            .chars()
            .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
    {
        return Err(Issue::new("invalidProjectNote"));
    }
    Ok((!note.is_empty()).then(|| note.to_owned()))
}

/// A project's startup file is named on the SAS command line, so it must be a
/// real file at an absolute path: there is no project folder to resolve a
/// relative path against, and SAS would read it from the sessions root instead.
fn validate_startup_file(path: Option<PathBuf>) -> Result<Option<PathBuf>, Issue> {
    let Some(path) = path else { return Ok(None) };
    if !path.is_absolute() {
        return Err(Issue::with_detail(
            "startupFileNotAbsolute",
            path.display().to_string(),
        ));
    }
    if !path.is_file() {
        return Err(Issue::with_detail(
            "startupFileMissing",
            path.display().to_string(),
        ));
    }
    Ok(Some(path))
}

/// SAS fills `SAS_EXECFILEPATH` only for programs the Enhanced Editor submits,
/// so a startup file that reads it gets an empty value unless the launch also
/// passes `-SET`. Refuse the launch rather than open a session whose paths are
/// silently wrong.
fn ensure_startup_path_available(startup: Option<&Path>, exposed: bool) -> Result<(), Issue> {
    let Some(startup) = startup else {
        return Ok(());
    };
    let bytes =
        std::fs::read(startup).map_err(|error| Issue::with_detail("startupFileMissing", error))?;
    let text = String::from_utf8_lossy(&bytes).to_ascii_uppercase();
    let uses_macro = text.contains(crate::process::STARTUP_PATH_VAR);
    let uses_env = text.contains("SAS_EXECFILEPATH");
    if uses_macro && !exposed {
        return Err(Issue::with_detail(
            "startupNeedsPath",
            startup.display().to_string(),
        ));
    }
    if uses_env && !uses_macro {
        return Err(Issue::with_detail(
            "startupNeedsMacro",
            startup.display().to_string(),
        ));
    }
    Ok(())
}

fn failure_stage(code: &str) -> Stage {
    match code {
        "sasMissing" | "invalidExe" | "exeAccess" | "dataPath" => Stage::Exe,
        "profileLocked"
        | "profileCopy"
        | "profileCreate"
        | "profileNotWritable"
        | "profileEmpty"
        | "sessionProfileCreate" => Stage::Profile,
        _ => Stage::Cfg,
    }
}

/// A declared layer the source folder cannot seed at all would be dropped by the
/// copy without a word, leaving a project that claims settings it never got.
fn ensure_source_seeds(source: &Path, layers: Layers) -> Result<(), Issue> {
    // An empty folder is not "a missing layer": say the plain thing first.
    if !profile::has_catalog_files(source) {
        return Err(Issue::new("profileSourceEmpty"));
    }
    match profile::first_unseeded(source, layers) {
        None => Ok(()),
        Some(code) => Err(Issue::new(code)),
    }
}

fn ensure_reference_exists(app: &App, reference: &ConfigRef) -> Result<(), Issue> {
    match reference {
        ConfigRef::BuiltIn { code } => {
            if app
                .factory_configs()
                .iter()
                .any(|config| config.code == *code)
            {
                Ok(())
            } else {
                Err(Issue::with_detail("configUnavailable", code))
            }
        }
        ConfigRef::User { id } => {
            if app.store.configuration(*id).is_some() {
                Ok(())
            } else {
                Err(Issue::new("configUnavailable"))
            }
        }
    }
}

#[tauri::command]
async fn snapshot(state: State<'_, AppState>) -> Result<View, Issue> {
    with_app(&state, |app| Ok(app.view()))
}

#[tauri::command]
async fn refresh_configs(state: State<'_, AppState>) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.refresh();
        Ok(app.view())
    })
}

#[tauri::command]
async fn select_installation(state: State<'_, AppState>, path: PathBuf) -> Result<View, Issue> {
    with_app(&state, |app| {
        let path = discovery::validate_exe(&path)?;
        app.store.update(|settings| settings.exe = Some(path))?;
        app.refresh();
        Ok(app.view())
    })
}

#[tauri::command]
async fn set_startup(state: State<'_, AppState>, enabled: bool) -> Result<View, Issue> {
    let exe =
        std::env::current_exe().map_err(|error| Issue::with_detail("autostartPath", error))?;
    let minimized = with_app(&state, |app| Ok(app.store.settings.start_minimized))?;
    autostart::set(&exe, enabled, minimized)?;
    let registered = autostart::enabled();
    with_app(&state, |app| {
        app.autostart = registered;
        Ok(app.view())
    })
}

#[tauri::command]
async fn set_start_minimized(state: State<'_, AppState>, enabled: bool) -> Result<View, Issue> {
    let registered = with_app(&state, |app| {
        app.store
            .update(|settings| settings.start_minimized = enabled)?;
        Ok(app.autostart)
    })?;
    // Keep the stored login command in step with the flag it carries.
    if registered {
        let exe =
            std::env::current_exe().map_err(|error| Issue::with_detail("autostartPath", error))?;
        autostart::set(&exe, true, enabled)?;
    }
    with_app(&state, |app| Ok(app.view()))
}

#[tauri::command]
async fn set_close_to_tray(state: State<'_, AppState>, enabled: bool) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.store
            .update(|settings| settings.close_to_tray = enabled)?;
        Ok(app.view())
    })
}

#[tauri::command]
async fn quit(handle: AppHandle) {
    handle.exit(0);
}

#[tauri::command]
async fn set_language(
    handle: AppHandle,
    state: State<'_, AppState>,
    preference: LanguagePreference,
) -> Result<View, Issue> {
    let (view, labels) = with_app(&state, |app| {
        app.store
            .update(|settings| settings.language = preference)?;
        Ok((app.view(), tray_labels(app)))
    })?;
    refresh_tray_menu(&handle, &labels);
    Ok(view)
}

#[tauri::command]
async fn set_theme(
    handle: AppHandle,
    state: State<'_, AppState>,
    preference: ThemePreference,
) -> Result<View, Issue> {
    let view = with_app(&state, |app| {
        app.store.update(|settings| settings.theme = preference)?;
        Ok(app.view())
    })?;
    if let Some(window) = handle.get_webview_window("main") {
        let _ = window.set_theme(preference.window_theme());
    }
    Ok(view)
}

/// Drop the manually pinned executable and fall back to scanning the machine.
#[tauri::command]
async fn clear_installation(state: State<'_, AppState>) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.store.update(|settings| settings.exe = None)?;
        app.refresh();
        Ok(app.view())
    })
}

/// What the form asked for. `Keep` leaves an existing project's snapshot alone,
/// which is what an edit that only renames a project sends.
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
enum ProfileChoice {
    Keep,
    SasUser { path: PathBuf, layers: Layers },
    ConfigSnapshot { config: Uuid, layers: Layers },
    Fresh { layers: Layers },
}

#[tauri::command]
// The arguments are the project form's fields, one per input; grouping them
// into a struct would only move the same list behind a name.
#[allow(clippy::too_many_arguments)]
async fn save_project(
    state: State<'_, AppState>,
    id: Option<Uuid>,
    name: String,
    note: Option<String>,
    config: Option<ConfigRef>,
    profile: Option<ProfileChoice>,
    startup_file: Option<PathBuf>,
    startup_exposes_path: bool,
) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.save_project(
            id,
            name,
            note,
            config,
            profile,
            startup_file,
            startup_exposes_path,
        )?;
        Ok(app.view())
    })
}

impl App {
    /// Save a project and materialise the preference snapshot it asked for.
    ///
    /// The row is written first because the capture commits its revision
    /// through `projects.json`; a failed capture puts the row back instead of
    /// leaving a project that claims to inherit settings it never copied.
    /// True when the stored snapshot no longer holds catalogs its declared layers
    /// need — a snapshot captured before a layer was turned on, or a pruned one.
    fn snapshot_lacks(&self, id: Uuid, profile: &ProjectProfile) -> bool {
        let Some(revision) = profile.revision else {
            return false;
        };
        !profile::unseeded_files(
            &self.store.project_profile_dir(id, revision),
            profile.layers,
        )
        .is_empty()
    }

    /// Where a project's snapshot came from, so a repair can copy the same folder.
    fn origin_source(&self, profile: &ProjectProfile) -> Option<PathBuf> {
        match profile.origin.as_ref()? {
            ProfileOrigin::SasUser { path } => Some(path.clone()),
            ProfileOrigin::ConfigSnapshot { config } => self
                .store
                .configuration(*config)
                .and_then(|configuration| self.store.profile_revision_dir(configuration)),
            ProfileOrigin::Fresh => None,
        }
    }

    /// See `save_project`'s command wrapper for why this signature is long.
    #[allow(clippy::too_many_arguments)]
    fn save_project(
        &mut self,
        id: Option<Uuid>,
        name: String,
        note: Option<String>,
        config: Option<ConfigRef>,
        choice: Option<ProfileChoice>,
        startup_file: Option<PathBuf>,
        startup_exposes_path: bool,
    ) -> Result<(), Issue> {
        let name = validate_name(&name)?;
        let note = validate_note(note)?;
        let startup_file = validate_startup_file(startup_file)?;
        let id = match id {
            Some(id) if self.store.projects().iter().any(|project| project.id == id) => id,
            Some(_) => return Err(Issue::new("projectMissing")),
            None => Uuid::new_v4(),
        };
        if let Some(reference) = config.as_ref() {
            ensure_reference_exists(self, reference)?;
        }
        let previous = self
            .store
            .projects()
            .iter()
            .find(|project| project.id == id)
            .cloned();
        let (profile, source) = match choice {
            None | Some(ProfileChoice::Keep) => {
                let kept = previous
                    .as_ref()
                    .and_then(|project| project.profile.clone());
                // A snapshot captured before a layer was turned on would answer
                // "keep" forever, so its missing catalogs are filled in while the
                // user is saving anyway — but only when the origin can seed them:
                // a locked folder must never block a rename.
                match kept {
                    Some(profile) if self.snapshot_lacks(id, &profile) => {
                        let layers = profile.layers;
                        match self.origin_source(&profile) {
                            Some(source) if profile::unseeded_files(&source, layers).is_empty() => {
                                (
                                    Some(ProjectProfile {
                                        revision: None,
                                        ..profile
                                    }),
                                    Some(source),
                                )
                            }
                            _ => (Some(profile), None),
                        }
                    }
                    kept => (kept, None),
                }
            }
            Some(ProfileChoice::Fresh { layers }) => (
                Some(ProjectProfile {
                    revision: None,
                    origin: Some(ProfileOrigin::Fresh),
                    layers,
                    fresh: true,
                }),
                None,
            ),
            Some(ProfileChoice::SasUser { path, layers }) => {
                ensure_source_seeds(&path, layers)?;
                (
                    Some(ProjectProfile {
                        revision: None,
                        origin: Some(ProfileOrigin::SasUser { path: path.clone() }),
                        layers,
                        fresh: false,
                    }),
                    Some(path),
                )
            }
            Some(ProfileChoice::ConfigSnapshot { config, layers }) => {
                let reference = ConfigRef::User { id: config };
                ensure_reference_exists(self, &reference)?;
                let source = self
                    .store
                    .configuration(config)
                    .and_then(|configuration| self.store.profile_revision_dir(configuration))
                    .filter(|directory| profile::has_catalog_files(directory))
                    .ok_or_else(|| Issue::new("profileSnapshotEmpty"))?;
                ensure_source_seeds(&source, layers)?;
                (
                    Some(ProjectProfile {
                        revision: None,
                        origin: Some(ProfileOrigin::ConfigSnapshot { config }),
                        layers,
                        fresh: false,
                    }),
                    Some(source),
                )
            }
        };
        let layers = profile
            .as_ref()
            .map(|profile| profile.layers)
            .unwrap_or_default();
        self.store.save_project(Project {
            id,
            name,
            note,
            config,
            profile,
            startup_file,
            startup_exposes_path,
        })?;
        let Some(source) = source else {
            return Ok(());
        };
        if let Err(error) =
            crate::store::capture_project_profile(&self.store.data_dir(), id, &source, Some(layers))
        {
            match previous {
                Some(previous) => {
                    let _ = self.store.save_project(previous);
                }
                None => {
                    let _ = self.store.delete_project(id);
                }
            }
            return Err(error);
        }
        self.store.reload()
    }
}

/// Version folders under `Documents\My SAS Files` that actually hold catalogs,
/// plus an optional folder the user picked themselves.
#[tauri::command]
async fn sasuser_candidates(
    state: State<'_, AppState>,
    extra: Option<PathBuf>,
) -> Result<Vec<sasuser::Candidate>, Issue> {
    with_app(&state, |app| {
        let preferred = app
            .installation
            .as_ref()
            .and_then(|installation| sasuser::version_from_exe(&installation.exe));
        let mut candidates = sasuser::candidates(preferred.as_deref());
        if let Some(extra) = extra.as_deref()
            && let Some(candidate) = sasuser::candidate_for(extra)
            && !candidates.iter().any(|known| known.path == candidate.path)
        {
            candidates.insert(0, candidate);
        }
        Ok(candidates)
    })
}

#[tauri::command]
async fn delete_project(state: State<'_, AppState>, id: Uuid) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.store.delete_project(id)?;
        Ok(app.view())
    })
}

/// Rewrite a project's startup script so it reads the macro variable the session
/// provides. The original stays next to it as `<name>.bak`.
#[tauri::command]
async fn migrate_startup_file(state: State<'_, AppState>, id: Uuid) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.migrate_startup_file(id)?;
        Ok(app.view())
    })
}

impl App {
    /// Rewrite the project's startup script in place, keeping the original as
    /// `<name>.bak`. The project is marked as exposing the path because the
    /// migrated script reads the variable the session only provides then.
    fn migrate_startup_file(&mut self, id: Uuid) -> Result<(), Issue> {
        self.refresh();
        let project = self
            .store
            .projects()
            .iter()
            .find(|project| project.id == id)
            .cloned()
            .ok_or_else(|| Issue::new("projectMissing"))?;
        let path = project
            .startup_file
            .clone()
            .ok_or_else(|| Issue::new("startupFileMissing"))?;
        let bytes = std::fs::read(&path)
            .map_err(|error| Issue::with_detail("startupFileMissing", error))?;
        // Byte-level: the script may be GBK, and decoding it as UTF-8 would
        // replace its Chinese folder names before writing it back.
        let migrated = crate::startup::migrate_bytes(&bytes).map_err(|error| match error {
            crate::startup::MigrateError::NothingToDo => {
                Issue::with_detail("startupAlreadyMigrated", path.display().to_string())
            }
            crate::startup::MigrateError::Unsupported => {
                Issue::with_detail("startupMigrateUnsupported", path.display().to_string())
            }
        })?;
        let backup = path.with_file_name(format!(
            "{}.bak",
            path.file_name().unwrap_or_default().to_string_lossy()
        ));
        if !backup.exists() {
            std::fs::copy(&path, &backup)
                .map_err(|error| Issue::with_detail("startupWrite", error))?;
        }
        std::fs::write(&path, migrated)
            .map_err(|error| Issue::with_detail("startupWrite", error))?;
        let mut updated = project;
        updated.startup_exposes_path = true;
        self.store.save_project(updated)
    }
}

#[tauri::command]
async fn create_configuration(
    state: State<'_, AppState>,
    base: ConfigBase,
    name: Option<String>,
) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.create_configuration(&base, name)?;
        Ok(app.view())
    })
}

#[tauri::command]
async fn update_configuration(
    state: State<'_, AppState>,
    id: Uuid,
    encoding: String,
    locale: String,
) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.update_configuration(id, encoding, locale)?;
        Ok(app.view())
    })
}

#[tauri::command]
async fn delete_configuration(state: State<'_, AppState>, id: Uuid) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.delete_configuration(id)?;
        Ok(app.view())
    })
}

#[tauri::command]
async fn start_config_setup(state: State<'_, AppState>, id: Uuid) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.start_config_setup(id)?;
        Ok(app.view())
    })
}

#[tauri::command]
async fn finish_config_setup(
    state: State<'_, AppState>,
    id: Uuid,
    save: bool,
) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.finish_config_setup(id, save)?;
        Ok(app.view())
    })
}

#[tauri::command]
async fn launch_project(state: State<'_, AppState>, id: Uuid) -> Result<SessionView, Issue> {
    with_app(&state, |app| {
        let record = app.launch_project(id)?;
        Ok(app.session_view(&record))
    })
}

#[tauri::command]
async fn launch_batch(
    state: State<'_, AppState>,
    ids: Vec<Uuid>,
) -> Result<Vec<BatchResult>, Issue> {
    with_app(&state, |app| Ok(app.launch_batch(ids)))
}

#[tauri::command]
async fn clear_diagnostics(state: State<'_, AppState>) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.clear_diagnostics()?;
        Ok(app.view())
    })
}

#[tauri::command]
async fn open_log(state: State<'_, AppState>, id: Uuid) -> Result<(), Issue> {
    with_app(&state, |app| open_path(&app.session_path(id, false)?))
}

#[tauri::command]
async fn open_sasuser(state: State<'_, AppState>, id: Uuid) -> Result<(), Issue> {
    with_app(&state, |app| open_path(&app.session_path(id, true)?))
}

#[tauri::command]
async fn open_archive(state: State<'_, AppState>) -> Result<(), Issue> {
    with_app(&state, |app| {
        let path = app
            .store
            .archived()
            .map(Path::to_path_buf)
            .ok_or_else(|| Issue::new("pathUnavailable"))?;
        open_path(&path)
    })
}

#[tauri::command]
async fn open_repository() -> Result<(), Issue> {
    open_url(HOME_PAGE)
}

#[tauri::command]
async fn open_issues() -> Result<(), Issue> {
    open_url(ISSUES_PAGE)
}

#[tauri::command]
async fn open_release(tag: String) -> Result<(), Issue> {
    open_url(&release_url(&tag)?)
}

/// The one tray icon this app owns.
const TRAY_ID: &str = "main";
/// Reveal the main window, whether it is hidden in the tray or minimized.
fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}

/// Native menu labels cannot come from the frontend catalogue, so they are read from
/// shared/labels.json through the same language preference the window uses.
fn tray_labels(app: &App) -> (String, String) {
    let language = app.store.settings.language.resolve(&app.system_language);
    (
        labels::text(language, "tray.showWindow"),
        labels::text(language, "tray.quit"),
    )
}

fn tray_menu(app: &AppHandle, labels: &(String, String)) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", &labels.0, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", &labels.1, true, None::<&str>)?;
    Menu::with_items(app, &[&show, &quit])
}

fn refresh_tray_menu(app: &AppHandle, labels: &(String, String)) {
    if let Some(tray) = app.tray_by_id(TRAY_ID)
        && let Ok(menu) = tray_menu(app, labels)
    {
        let _ = tray.set_menu(Some(menu));
    }
}

fn running_sessions(app: &App) -> usize {
    app.sessions
        .cached()
        .iter()
        .filter(|record| record.status == Status::Running)
        .count()
}

/// What the window's close button should do, decided where the session state
/// is first-hand rather than in the polled frontend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CloseIntent {
    /// Keep running in the tray.
    Hide,
    /// Ask before leaving SAS sessions behind.
    Ask,
    /// Close the window and let the app exit.
    Quit,
}

fn close_intent(close_to_tray: bool, running: usize) -> CloseIntent {
    match (close_to_tray, running) {
        (true, _) => CloseIntent::Hide,
        (false, 0) => CloseIntent::Quit,
        (false, _) => CloseIntent::Ask,
    }
}

/// Quitting with SAS still running would strand those sessions: ask first.
fn request_quit(app: &AppHandle) {
    let running =
        with_app(&app.state::<AppState>(), |app| Ok(running_sessions(app))).unwrap_or_default();
    if running == 0 {
        app.exit(0);
        return;
    }
    show_main_window(app);
    let _ = app.emit("quit-requested", ());
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            show_main_window(app);
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state: AppState = Mutex::new(match app.path().app_local_data_dir() {
                Ok(root) => App::new(root),
                Err(error) => Err(Issue::with_detail("dataPath", error)),
            });
            app.manage(state);
            let registered = autostart::enabled();
            let _ = with_app(&app.state::<AppState>(), |app| {
                app.autostart = registered;
                Ok(())
            });
            if let Some(window) = app.get_webview_window("main") {
                let theme = with_app(&app.state::<AppState>(), |app| Ok(app.store.settings.theme))
                    .unwrap_or_default();
                let _ = window.set_theme(theme.window_theme());
            }
            if autostart::requested_minimized(&std::env::args().collect::<Vec<_>>())
                && let Some(window) = app.get_webview_window("main")
            {
                let _ = window.hide();
            }
            let labels = with_app(&app.state::<AppState>(), |app| Ok(tray_labels(app)))
                .unwrap_or_else(|_| ("Show window".to_owned(), "Quit".to_owned()));
            if let Some(icon) = app.default_window_icon().cloned() {
                let menu = tray_menu(app.handle(), &labels)?;
                TrayIconBuilder::with_id(TRAY_ID)
                    .icon(icon)
                    .tooltip("SAS Launcher")
                    .menu(&menu)
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id.as_ref() {
                        "show" => show_main_window(app),
                        "quit" => request_quit(app),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| {
                        if let TrayIconEvent::Click {
                            button: MouseButton::Left,
                            button_state: MouseButtonState::Up,
                            ..
                        } = event
                        {
                            show_main_window(tray.app_handle());
                        }
                    })
                    .build(app)?;
            }
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    let state = handle.state::<AppState>();
                    let _ = with_app(&state, |app| {
                        app.maintenance();
                        Ok(())
                    });
                    std::thread::sleep(Duration::from_secs(3));
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            let WindowEvent::CloseRequested { api, .. } = event else {
                return;
            };
            let (close_to_tray, running) = with_app(&window.state::<AppState>(), |app| {
                Ok((app.store.settings.close_to_tray, running_sessions(app)))
            })
            .unwrap_or((true, 0));
            match close_intent(close_to_tray, running) {
                CloseIntent::Hide => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                CloseIntent::Ask => {
                    // The window stays up; the confirmation owns the exit.
                    api.prevent_close();
                    let _ = window.app_handle().emit("quit-requested", ());
                }
                CloseIntent::Quit => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            snapshot,
            sasuser_candidates,
            refresh_configs,
            select_installation,
            set_language,
            set_theme,
            set_close_to_tray,
            set_startup,
            set_start_minimized,
            quit,
            clear_installation,
            save_project,
            delete_project,
            migrate_startup_file,
            create_configuration,
            update_configuration,
            delete_configuration,
            start_config_setup,
            finish_config_setup,
            launch_project,
            launch_batch,
            clear_diagnostics,
            open_log,
            open_sasuser,
            open_archive,
            open_repository,
            open_issues,
            open_release
        ])
        .run(tauri::generate_context!())
        .expect("Could not run SAS Launcher");
}

#[cfg(test)]
mod tests {
    use super::*;
    use configuration::PresetStatus;

    fn install_fixture(root: &Path, codes: &[(&str, &str, &str)], default: &str) -> PathBuf {
        let install = root.join("install");
        std::fs::create_dir_all(&install).unwrap();
        let exe = install.join("sas.exe");
        std::fs::write(&exe, b"fixture").unwrap();
        for (code, encoding, locale) in codes {
            let directory = install.join("nls").join(code);
            std::fs::create_dir_all(&directory).unwrap();
            std::fs::write(
                directory.join("sasv9.cfg"),
                format!("-ENCODING {encoding}\r\n-LOCALE {locale}\r\n"),
            )
            .unwrap();
        }
        std::fs::write(
            install.join("sasv9.cfg"),
            format!("-CONFIG \"!SASROOT\\nls\\{default}\\sasv9.cfg\""),
        )
        .unwrap();
        exe
    }

    #[test]
    fn closing_hides_asks_or_quits_by_setting_and_running_sessions() {
        assert!(matches!(close_intent(true, 0), CloseIntent::Hide));
        assert!(matches!(close_intent(true, 3), CloseIntent::Hide));
        assert!(matches!(close_intent(false, 0), CloseIntent::Quit));
        assert!(matches!(close_intent(false, 1), CloseIntent::Ask));
    }

    #[test]
    fn a_running_setup_session_counts_for_the_close_prompt() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        record_session(
            &app,
            Purpose::Setup {
                config: Uuid::new_v4(),
            },
            Status::Running,
            30,
        );
        // The cache is refreshed by the maintenance sweep, as it is at runtime.
        app.maintenance();

        assert_eq!(running_sessions(&app), 1);
    }

    fn app_with_install(root: &Path, exe: &Path) -> App {
        // `App::new` falls back to `discovery::find_installed` when settings pin
        // no executable, so an unpinned launch test starts the machine's real
        // SAS. A launch test therefore has to point at a fixture install that
        // lives under its own root.
        assert!(
            exe.starts_with(root),
            "launch tests must pin a fixture install, not {}",
            exe.display()
        );
        let mut app = App::new(root.join("data")).unwrap();
        app.store
            .update(|settings| settings.exe = Some(exe.to_path_buf()))
            .unwrap();
        app.refresh();
        app
    }

    /// Write a session manifest the loader will treat as `status`, which is how
    /// a launch in progress survives an app restart.
    fn record_session(app: &App, purpose: Purpose, status: Status, started_at: i64) -> Uuid {
        let id = Uuid::new_v4();
        let directory = app.store.sessions_dir().join(id.to_string());
        std::fs::create_dir_all(&directory).unwrap();
        let record = Record {
            id,
            name: format!("session-{started_at}"),
            title: format!("session-{started_at}"),
            purpose,
            stage: Stage::Started,
            status,
            exit_code: None,
            started_at,
            argv: Vec::new(),
            sasuser: directory.join("sasuser"),
            sasuser_writable: true,
            profile_files: Vec::new(),
            thin_cfg: None,
            factory_cfg: PathBuf::from(r"C:\SAS\sasv9.cfg"),
            encoding: Some("UTF-8".into()),
            locale: Some("zh_CN".into()),
            failure: None,
        };
        // A manifest is the flattened record plus the liveness identity; a
        // missing process is probed from `identity`, so `null` keeps the status.
        let mut manifest = serde_json::to_value(&record).unwrap();
        manifest["identity"] = serde_json::Value::Null;
        manifest["exit_recorded"] = serde_json::Value::Bool(false);
        std::fs::write(
            directory.join("manifest.json"),
            serde_json::to_string(&manifest).unwrap(),
        )
        .unwrap();
        id
    }

    fn save_project(app: &mut App, name: &str) -> Uuid {
        app.store
            .save_project(Project {
                id: Uuid::new_v4(),
                name: name.into(),
                note: None,
                config: None,
                profile: None,
                startup_file: None,
                startup_exposes_path: false,
            })
            .unwrap();
        app.store
            .projects()
            .iter()
            .find(|project| project.name == name)
            .map(|project| project.id)
            .unwrap()
    }

    #[test]
    fn a_project_that_is_already_running_is_not_launched_again() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        let live = save_project(&mut app, "Live");
        record_session(
            &app,
            Purpose::Project { id: Some(live) },
            Status::Running,
            20,
        );

        let error = app.launch_project(live).unwrap_err();
        assert_eq!(error.code, "projectRunning");
        let records = app.sessions.cached();
        assert_eq!(records.len(), 1, "a refused launch leaves no new record");
        assert_eq!(records[0].status, Status::Running);

        // An exited session must not block the next launch: the guard lets it
        // through, so the failure comes from the fixture's unusable sas.exe.
        let idle = save_project(&mut app, "Idle");
        record_session(
            &app,
            Purpose::Project { id: Some(idle) },
            Status::Exited,
            10,
        );
        let error = app.launch_project(idle).unwrap_err();
        assert_ne!(error.code, "projectRunning");
    }

    #[test]
    fn one_projects_session_does_not_block_another_project() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        let busy = save_project(&mut app, "Busy");
        let other = save_project(&mut app, "Other");
        record_session(
            &app,
            Purpose::Project { id: Some(busy) },
            Status::Running,
            20,
        );

        let error = app.launch_project(other).unwrap_err();
        assert_ne!(error.code, "projectRunning");
    }

    #[test]
    #[should_panic(expected = "launch tests must pin a fixture install")]
    fn a_launch_fixture_refuses_an_install_outside_its_data_root() {
        let temp = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let exe = install_fixture(elsewhere.path(), &[("en", "wlatin1", "en_US")], "en");
        let _ = app_with_install(temp.path(), &exe);
    }

    #[test]
    fn a_session_view_names_the_project_it_belongs_to() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        let project = save_project(&mut app, "Live");
        let session = record_session(
            &app,
            Purpose::Project { id: Some(project) },
            Status::Running,
            20,
        );
        let setup = record_session(
            &app,
            Purpose::Setup {
                config: Uuid::new_v4(),
            },
            Status::Running,
            21,
        );

        let view = app.view();
        let running = view
            .sessions
            .iter()
            .find(|row| row.id == session)
            .expect("the project session is in the view");
        assert_eq!(
            running.project_id,
            Some(project),
            "the page matches a row to its session by project id, not by name"
        );
        let setup_row = view
            .sessions
            .iter()
            .find(|row| row.id == setup)
            .expect("the settings session is in the view");
        assert_eq!(setup_row.project_id, None);
    }

    #[test]
    fn deriving_from_user_config_copies_options_and_profile() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(
            temp.path(),
            &[("u8", "UTF-8", "zh_CN"), ("en", "wlatin1", "en_US")],
            "u8",
        );
        let mut app = app_with_install(temp.path(), &exe);
        // The generated name follows the interface language, so pin it before deriving.
        app.store
            .update(|settings| settings.language = LanguagePreference::ZhHans)
            .unwrap();
        let factory = discovery::factory_cfg_for(&exe, "u8").unwrap();
        let profile = temp.path().join("profile");
        std::fs::create_dir_all(&profile).unwrap();
        std::fs::write(profile.join("profile.sas7bcat"), b"baseline").unwrap();
        std::fs::write(profile.join("regstry.sas7bitm"), b"registry").unwrap();
        let base_id = app
            .store
            .create_configuration(store::CreateConfiguration {
                name: "Base".into(),
                base: ConfigBase::BuiltIn { code: "u8".into() },
                factory: factory.clone(),
                encoding: "UTF-8".into(),
                locale: "zh_CN".into(),
                profile: Some(profile.clone()),
            })
            .unwrap();

        let derived_id = app
            .create_configuration(&ConfigBase::User { id: base_id }, None)
            .unwrap();

        let base = app.store.configuration(base_id).unwrap().clone();
        let derived = app.store.configuration(derived_id).unwrap().clone();
        assert_eq!(derived.encoding, base.encoding);
        assert_eq!(derived.locale, base.locale);
        assert_eq!(derived.factory, base.factory);
        assert_ne!(derived.id, base.id);
        assert_eq!(
            derived.name, "用户配置（衍生自 Base）#1",
            "default name numbers per source"
        );
        let profile_dir = app.store.profile_revision_dir(&derived).unwrap();
        assert_eq!(
            std::fs::read(profile_dir.join("profile.sas7bcat")).unwrap(),
            b"baseline"
        );
        assert_eq!(
            std::fs::read(profile_dir.join("regstry.sas7bitm")).unwrap(),
            b"registry"
        );
        // The thin cfg points at the factory cfg, never at the other thin cfg.
        let contents = std::fs::read_to_string(app.store.thin_cfg_path(&derived)).unwrap();
        assert!(contents.contains("nls\\u8\\sasv9.cfg"), "{contents}");
        let view = app.view();
        let row = view
            .configs
            .iter()
            .find(|row| row.reference == ConfigRef::User { id: derived_id })
            .unwrap();
        assert_eq!(row.derived_from.as_deref(), Some("Base"));
        assert!(row.has_profile);
        assert_eq!(row.preset, PresetStatus::Verified);
    }

    #[test]
    fn deleting_a_referenced_configuration_is_refused() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        let id = app
            .create_configuration(&ConfigBase::BuiltIn { code: "en".into() }, None)
            .unwrap();
        let project_id = Uuid::new_v4();
        app.store
            .save_project(Project {
                id: project_id,
                name: "Bound".into(),
                note: None,
                config: Some(ConfigRef::User { id }),
                profile: None,
                startup_file: None,
                startup_exposes_path: false,
            })
            .unwrap();

        let error = app.delete_configuration(id).unwrap_err();
        assert_eq!(error.code, "configurationInUse");
        assert_eq!(error.detail.as_deref(), Some("1"));

        app.store
            .save_project(Project {
                id: project_id,
                name: "Bound".into(),
                note: None,
                config: None,
                profile: None,
                startup_file: None,
                startup_exposes_path: false,
            })
            .unwrap();
        app.delete_configuration(id).unwrap();
        assert!(app.store.configuration(id).is_none());
    }

    #[test]
    fn project_resolves_default_builtin_and_user_references() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(
            temp.path(),
            &[("en", "wlatin1", "en_US"), ("u8", "UTF-8", "zh_CN")],
            "en",
        );
        let mut app = app_with_install(temp.path(), &exe);

        let default = app.resolve(None).unwrap();
        assert!(
            default.cfg.ends_with("nls\\en\\sasv9.cfg"),
            "{:?}",
            default.cfg
        );
        assert_eq!(default.encoding.as_deref(), Some("wlatin1"));
        assert_eq!(default.locale.as_deref(), Some("en_US"));
        assert!(default.thin.is_none());

        let built_in = app
            .resolve(Some(&ConfigRef::BuiltIn { code: "u8".into() }))
            .unwrap();
        assert!(built_in.cfg.ends_with("nls\\u8\\sasv9.cfg"));
        assert_eq!(built_in.encoding.as_deref(), Some("UTF-8"));

        let id = app
            .create_configuration(
                &ConfigBase::BuiltIn { code: "u8".into() },
                Some("Mine".into()),
            )
            .unwrap();
        let user = app.resolve(Some(&ConfigRef::User { id })).unwrap();
        assert_eq!(
            user.cfg,
            app.store
                .thin_cfg_path(app.store.configuration(id).unwrap())
        );
        assert!(user.thin.is_some());
        assert_eq!(user.factory, built_in.cfg);
        assert_eq!(user.encoding.as_deref(), Some("UTF-8"));
        assert_eq!(user.locale.as_deref(), Some("zh_CN"));
    }

    #[test]
    fn missing_builtin_code_reports_config_unavailable() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let app = app_with_install(temp.path(), &exe);

        let error = app
            .resolve(Some(&ConfigRef::BuiltIn {
                code: "missing".into(),
            }))
            .unwrap_err();
        assert_eq!(error.code, "configUnavailable");

        // Saving a project that points at a missing built-in code is refused.
        let error = ensure_reference_exists(
            &app,
            &ConfigRef::BuiltIn {
                code: "missing".into(),
            },
        )
        .unwrap_err();
        assert_eq!(error.code, "configUnavailable");
    }

    #[test]
    fn derives_from_a_factory_locale_that_is_a_bare_language_name() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("es", "wlatin1", "Spanish")], "es");
        let mut app = app_with_install(temp.path(), &exe);

        let id = app
            .create_configuration(&ConfigBase::BuiltIn { code: "es".into() }, None)
            .expect("the factory Spanish locale must be accepted");
        let config = app.store.configuration(id).unwrap().clone();
        assert_eq!(config.locale, "Spanish");
        let contents = std::fs::read_to_string(app.store.thin_cfg_path(&config)).unwrap();
        assert!(contents.contains("-LOCALE Spanish"), "{contents}");
    }

    #[test]
    fn a_launch_that_cannot_resolve_still_appears_in_diagnostics() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        app.store
            .save_project(Project {
                id: Uuid::new_v4(),
                name: "Broken".into(),
                note: None,
                config: Some(ConfigRef::BuiltIn {
                    code: "missing".into(),
                }),
                profile: None,
                startup_file: None,
                startup_exposes_path: false,
            })
            .unwrap();
        let id = app.store.projects()[0].id;

        let error = app.launch_project(id).unwrap_err();
        assert_eq!(error.code, "configUnavailable");

        let records = app.sessions.cached();
        assert_eq!(records.len(), 1, "the failed launch must leave evidence");
        assert_eq!(records[0].stage, Stage::Cfg);
        assert_eq!(
            records[0].failure.as_ref().map(|note| note.code.as_str()),
            Some("configUnavailable")
        );
    }

    #[test]
    fn a_startup_file_that_locates_itself_is_refused_until_the_project_allows_it() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        let setup = temp.path().join("setup.sas");
        std::fs::write(&setup, b"%let DIR = &SASLAUNCHER_STARTUP_FILE;\n").unwrap();
        app.save_project(
            None,
            "P".into(),
            None,
            None,
            None,
            Some(setup.clone()),
            false,
        )
        .unwrap();
        let id = app.store.projects()[0].id;

        let error = app.launch_project(id).unwrap_err();
        assert_eq!(error.code, "startupNeedsPath");
        let records = app.sessions.cached();
        assert_eq!(records.len(), 1, "the refused launch must leave evidence");
        assert!(records[0].argv.is_empty(), "{:?}", records[0].argv);

        // Allowing the exposure carries the same project past the guard.
        app.save_project(Some(id), "P".into(), None, None, None, Some(setup), true)
            .unwrap();
        let later = app.launch_project(id).unwrap_err();
        assert_ne!(later.code, "startupNeedsPath");
    }

    #[test]
    fn a_startup_file_that_still_reads_the_environment_variable_is_refused() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        let setup = temp.path().join("setup.sas");
        // SAS resets that variable during a windowed start, so instructing the
        // project to expose the path cannot make this script work.
        std::fs::write(&setup, b"%let DIR = %sysget(SAS_EXECFILEPATH);\n").unwrap();
        app.save_project(None, "P".into(), None, None, None, Some(setup), true)
            .unwrap();
        let id = app.store.projects()[0].id;

        let error = app.launch_project(id).unwrap_err();
        assert_eq!(error.code, "startupNeedsMacro");
        assert!(app.sessions.cached()[0].argv.is_empty());
    }

    #[test]
    fn migrating_a_startup_script_keeps_the_original_and_lets_the_launch_through() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        let setup = temp.path().join("setup.sas");
        let original = "%let DIR_CURRENT = %sysget(SAS_EXECFILEPATH);\n%put &DIR_CURRENT;\n";
        std::fs::write(&setup, original).unwrap();
        app.save_project(
            None,
            "P".into(),
            None,
            None,
            None,
            Some(setup.clone()),
            false,
        )
        .unwrap();
        let id = app.store.projects()[0].id;

        app.migrate_startup_file(id).unwrap();

        let migrated = std::fs::read_to_string(&setup).unwrap();
        assert!(
            migrated.contains("%if %symexist(SASLAUNCHER_STARTUP_FILE) %then %do;"),
            "{migrated}"
        );
        assert!(
            migrated.contains("%let DIR_CURRENT = &SASLAUNCHER_STARTUP_FILE;"),
            "{migrated}"
        );
        assert!(migrated.ends_with("%put &DIR_CURRENT;\n"), "{migrated}");
        let backup = temp.path().join("setup.sas.bak");
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), original);
        assert!(app.store.projects()[0].startup_exposes_path);

        // The migrated script no longer trips the guard.
        let later = app.launch_project(id).unwrap_err();
        assert_ne!(later.code, "startupNeedsMacro");
    }

    #[test]
    fn migrating_an_already_migrated_script_changes_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        let setup = temp.path().join("setup.sas");
        let migrated = String::from_utf8(
            crate::startup::migrate_bytes(b"%let DIR = %sysget(SAS_EXECFILEPATH);\n").unwrap(),
        )
        .unwrap();
        std::fs::write(&setup, &migrated).unwrap();
        app.save_project(
            None,
            "P".into(),
            None,
            None,
            None,
            Some(setup.clone()),
            true,
        )
        .unwrap();
        let id = app.store.projects()[0].id;

        let error = app.migrate_startup_file(id).unwrap_err();
        assert_eq!(error.code, "startupAlreadyMigrated");
        assert_eq!(std::fs::read_to_string(&setup).unwrap(), migrated);
        assert!(!temp.path().join("setup.sas.bak").exists());
    }

    #[test]
    fn migrating_a_script_with_an_unsupported_shape_changes_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        let setup = temp.path().join("setup.sas");
        let original = "%let dir = %substr(%sysget(SAS_EXECFILEPATH), 1, 3);\n";
        std::fs::write(&setup, original).unwrap();
        app.save_project(
            None,
            "P".into(),
            None,
            None,
            None,
            Some(setup.clone()),
            true,
        )
        .unwrap();
        let id = app.store.projects()[0].id;

        let error = app.migrate_startup_file(id).unwrap_err();
        assert_eq!(error.code, "startupMigrateUnsupported");
        assert_eq!(std::fs::read_to_string(&setup).unwrap(), original);
        assert!(!temp.path().join("setup.sas.bak").exists());
    }

    #[test]
    fn a_panicked_command_does_not_disable_later_ones() {
        let state: AppState = Mutex::new(Err(Issue::new("internal")));
        let poisoned = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = state.lock().unwrap();
            panic!("a command panicked while holding the lock");
        }));
        assert!(poisoned.is_err());
        assert!(state.lock().is_err(), "the lock is poisoned");

        // Recovering answers instead of failing every command from then on.
        let guard = lock_app(&state);
        assert_eq!(
            guard.as_ref().err().map(|issue| issue.code),
            Some("internal")
        );
    }

    #[test]
    fn migrating_a_gbk_script_keeps_its_bytes_and_its_backup() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        let setup = temp.path().join("setup.sas");
        // "统计分析" in GBK, as an older project stores it.
        let chinese: [u8; 8] = [0xCD, 0xB3, 0xBC, 0xC6, 0xB7, 0xD6, 0xCE, 0xF6];
        let mut original = b"%let DIR_STAT = C:\\04 ".to_vec();
        original.extend_from_slice(&chinese);
        original.extend_from_slice(b";\r\n%let DIR = %sysget(SAS_EXECFILEPATH);\r\n");
        std::fs::write(&setup, &original).unwrap();
        app.save_project(
            None,
            "P".into(),
            None,
            None,
            None,
            Some(setup.clone()),
            false,
        )
        .unwrap();
        let id = app.store.projects()[0].id;

        app.migrate_startup_file(id).unwrap();

        let migrated = std::fs::read(&setup).unwrap();
        assert!(
            migrated
                .windows(chinese.len())
                .any(|window| window == chinese),
            "the gbk bytes survive the rewrite"
        );
        assert!(
            migrated
                .windows(40)
                .any(|window| window.starts_with(b"%if %symexist(SASLAUNCHER_STARTUP_FI")),
            "the guard block is inserted"
        );
        assert_eq!(
            std::fs::read(temp.path().join("setup.sas.bak")).unwrap(),
            original
        );
    }

    #[test]
    fn saving_a_project_whose_startup_file_is_missing_is_refused() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);

        let missing = temp.path().join("setup.sas");
        let error = app
            .save_project(None, "P".into(), None, None, None, Some(missing), false)
            .unwrap_err();
        assert_eq!(error.code, "startupFileMissing");
        assert!(
            error
                .detail
                .as_deref()
                .is_some_and(|detail| detail.ends_with("setup.sas")),
            "{error:?}"
        );
        assert!(
            app.store.projects().is_empty(),
            "a refused save stores nothing"
        );

        // A relative path has no project folder to resolve against, so it is
        // refused instead of being read against the sessions root by accident.
        let error = app
            .save_project(
                None,
                "P".into(),
                None,
                None,
                None,
                Some("setup.sas".into()),
                false,
            )
            .unwrap_err();
        assert_eq!(error.code, "startupFileNotAbsolute");
    }

    #[test]
    fn a_project_whose_startup_file_vanished_is_not_launched() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        let setup = temp.path().join("setup.sas");
        std::fs::write(&setup, b"%put setup;").unwrap();
        app.save_project(
            None,
            "P".into(),
            None,
            None,
            None,
            Some(setup.clone()),
            false,
        )
        .unwrap();
        let id = app.store.projects()[0].id;
        std::fs::remove_file(&setup).unwrap();

        let error = app.launch_project(id).unwrap_err();
        assert_eq!(error.code, "startupFileMissing");

        let records = app.sessions.cached();
        assert_eq!(records.len(), 1, "the refused launch must leave evidence");
        assert_eq!(records[0].stage, Stage::Cfg);
        assert_eq!(
            records[0].failure.as_ref().map(|note| note.code.as_str()),
            Some("startupFileMissing")
        );
        // Recorded before the command line existed: no process was spawned.
        assert!(records[0].argv.is_empty(), "{:?}", records[0].argv);
    }

    #[test]
    fn a_batched_launch_reports_a_project_without_its_startup_file() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        let setup = temp.path().join("setup.sas");
        std::fs::write(&setup, b"%put setup;").unwrap();
        app.save_project(
            None,
            "P".into(),
            None,
            None,
            None,
            Some(setup.clone()),
            false,
        )
        .unwrap();
        let id = app.store.projects()[0].id;
        std::fs::remove_file(&setup).unwrap();

        let results = app.launch_batch(vec![id]);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].status, "failed");
        assert_eq!(
            results[0].reason.as_ref().map(|issue| issue.code),
            Some("startupFileMissing")
        );
    }

    #[test]
    fn every_issue_code_used_in_rust_exists_in_issue_messages() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let messages_path = manifest.join("../src/issue-messages.json");
        let text = std::fs::read_to_string(&messages_path)
            .unwrap_or_else(|error| panic!("{}: {error}", messages_path.display()));
        let messages: serde_json::Value = serde_json::from_str(&text).unwrap();
        let messages = messages.as_object().expect("an object of codes");

        let mut codes: Vec<String> = Vec::new();
        let mut scanned = 0;
        for entry in std::fs::read_dir(manifest.join("src")).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("rs") {
                continue;
            }
            scanned += 1;
            let source = std::fs::read_to_string(&path).unwrap();
            for marker in ["Issue::new(\"", "Issue::with_detail(\""] {
                let mut rest = source.as_str();
                while let Some(index) = rest.find(marker) {
                    let after = &rest[index + marker.len()..];
                    if let Some(end) = after.find('"') {
                        let code = &after[..end];
                        if !code.is_empty() && !codes.iter().any(|known| known == code) {
                            codes.push(code.to_owned());
                        }
                    }
                    rest = after;
                }
            }
        }
        assert!(
            scanned >= 6,
            "expected the backend modules, scanned {scanned}"
        );
        assert!(codes.len() >= 20, "code scan found too few: {codes:?}");

        let missing: Vec<&String> = codes
            .iter()
            .filter(|code| !messages.contains_key(code.as_str()))
            .collect();
        assert!(
            missing.is_empty(),
            "issue codes emitted by Rust but missing from src/issue-messages.json: {missing:?}"
        );

        // Every entry carries all three languages, so no interface falls back silently.
        for (code, entry) in messages {
            let columns = entry
                .as_array()
                .unwrap_or_else(|| panic!("{code} is not an array of translations"));
            assert_eq!(
                columns.len(),
                3,
                "{code} should hold zh-Hans, en and zh-Hant"
            );
            for column in columns {
                let text = column
                    .as_str()
                    .unwrap_or_else(|| panic!("{code} holds a non-string translation"));
                assert!(!text.trim().is_empty(), "{code} has an empty translation");
            }
        }
        // The messages file itself must stay usable for the frontend.
        assert!(messages.contains_key("sasMissing"));
        assert!(messages.contains_key("configUnavailable"));
    }

    fn user_folder_with(temp: &Path, names: &[&str]) -> PathBuf {
        let folder = temp.join("My SAS Files").join("9.4");
        std::fs::create_dir_all(&folder).unwrap();
        for name in names {
            std::fs::write(folder.join(name), b"settings").unwrap();
        }
        folder
    }

    #[test]
    fn a_rename_save_fills_in_a_layer_the_snapshot_missed() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(temp.path().join("data")).unwrap();
        let source = user_folder_with(temp.path(), &["profile.sas7bcat", "regstry.sas7bitm"]);
        let interface_only = Layers {
            interface: true,
            environment: false,
            templates: false,
        };
        app.save_project(
            None,
            "P".into(),
            None,
            None,
            Some(ProfileChoice::SasUser {
                path: source.clone(),
                layers: interface_only,
            }),
            None,
            false,
        )
        .unwrap();
        // What the new default leaves behind on an old project: the declaration
        // asks for the environment layer while the snapshot has no catalog for it.
        let mut stored = app.store.projects()[0].clone();
        let mut profile = stored.profile.clone().unwrap();
        profile.layers = Layers::DEFAULT;
        stored.profile = Some(profile);
        app.store.save_project(stored.clone()).unwrap();

        app.save_project(
            Some(stored.id),
            "Renamed".into(),
            None,
            None,
            Some(ProfileChoice::Keep),
            None,
            false,
        )
        .unwrap();

        let profile = app.store.projects()[0].profile.clone().unwrap();
        let snapshot = app
            .store
            .project_profile_dir(stored.id, profile.revision.unwrap());
        assert!(
            snapshot.join("regstry.sas7bitm").is_file(),
            "keeping a project still fills in a layer its snapshot missed"
        );
    }

    #[test]
    fn a_source_without_the_chosen_layer_is_refused() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(temp.path().join("data")).unwrap();
        let source = user_folder_with(temp.path(), &["profile.sas7bcat"]);

        let error = app
            .save_project(
                None,
                "P".into(),
                None,
                None,
                Some(ProfileChoice::SasUser {
                    path: source.clone(),
                    layers: Layers::DEFAULT,
                }),
                None,
                false,
            )
            .unwrap_err();

        assert_eq!(error.code, "profileEnvironmentMissing");
        assert!(
            app.store.projects().is_empty(),
            "a layer the source cannot carry is never promised silently"
        );
    }

    #[test]
    fn an_empty_source_is_refused_before_any_layer_is_named() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(temp.path().join("data")).unwrap();
        let empty = temp.path().join("Desktop");
        std::fs::create_dir_all(&empty).unwrap();

        let error = app
            .save_project(
                None,
                "P".into(),
                None,
                None,
                Some(ProfileChoice::SasUser {
                    path: empty,
                    layers: Layers::DEFAULT,
                }),
                None,
                false,
            )
            .unwrap_err();

        assert_eq!(
            error.code, "profileSourceEmpty",
            "a folder with no SAS settings at all is not a missing layer"
        );
        assert!(app.store.projects().is_empty());
    }

    #[test]
    fn inheriting_a_configuration_that_saved_nothing_is_named_as_such() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        let config = app
            .create_configuration(&ConfigBase::BuiltIn { code: "en".into() }, None)
            .unwrap();

        let error = app
            .save_project(
                None,
                "P".into(),
                None,
                None,
                Some(ProfileChoice::ConfigSnapshot {
                    config,
                    layers: Layers::DEFAULT,
                }),
                None,
                false,
            )
            .unwrap_err();

        assert_eq!(
            error.code, "profileSnapshotEmpty",
            "the configuration flow keeps its own remedy, not the folder one"
        );
        assert!(app.store.projects().is_empty());
    }

    #[test]
    fn a_batched_launch_honours_the_projects_preference_snapshot() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(temp.path(), &[("en", "wlatin1", "en_US")], "en");
        let mut app = app_with_install(temp.path(), &exe);
        let source = user_folder_with(temp.path(), &["profile.sas7bcat", "regstry.sas7bitm"]);
        app.save_project(
            None,
            "P".into(),
            None,
            None,
            Some(ProfileChoice::SasUser {
                path: source.clone(),
                layers: Layers::DEFAULT,
            }),
            None,
            false,
        )
        .unwrap();
        let id = app.store.projects()[0].id;

        let results = app.launch_batch(vec![id]);
        assert_eq!(results.len(), 1);

        // The fixture's sas.exe cannot start, so the launch stops right after the
        // profile was seeded. Reading the folder straight off disk keeps the
        // assertion independent of the sweep that later reaps it.
        let mut entries: Vec<_> = std::fs::read_dir(app.store.sessions_dir())
            .unwrap()
            .flatten()
            .map(|entry| entry.path())
            .collect();
        entries.sort();
        assert_eq!(entries.len(), 1, "one session folder");
        assert!(
            entries[0]
                .join("sasuser")
                .join("profile.sas7bcat")
                .is_file(),
            "a batch launch seeds the project's own snapshot, like a single launch"
        );
    }

    #[test]
    fn a_new_project_inheriting_a_user_folder_captures_the_default_layers() {
        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(temp.path().join("data")).unwrap();
        let source = user_folder_with(temp.path(), &["profile.sas7bcat", "regstry.sas7bitm"]);

        app.save_project(
            None,
            "P".into(),
            None,
            None,
            Some(ProfileChoice::SasUser {
                path: source.clone(),
                layers: Layers::DEFAULT,
            }),
            None,
            false,
        )
        .unwrap();

        let project = &app.store.projects()[0];
        let profile = project.profile.as_ref().unwrap();
        assert!(matches!(
            profile.origin,
            Some(ProfileOrigin::SasUser { .. })
        ));
        let revision = profile.revision.unwrap();
        let snapshot = app.store.project_profile_dir(project.id, revision);
        assert!(snapshot.join("profile.sas7bcat").is_file());
        assert!(snapshot.join("regstry.sas7bitm").is_file());
    }

    #[test]
    fn an_unreadable_source_leaves_no_project_behind() {
        use std::os::windows::fs::OpenOptionsExt;

        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(temp.path().join("data")).unwrap();
        let source = user_folder_with(temp.path(), &["profile.sas7bcat", "regstry.sas7bitm"]);
        let locked = std::fs::File::options()
            .read(true)
            .share_mode(0)
            .open(source.join("profile.sas7bcat"))
            .unwrap();

        let error = app
            .save_project(
                None,
                "P".into(),
                None,
                None,
                Some(ProfileChoice::SasUser {
                    path: source.clone(),
                    layers: Layers::DEFAULT,
                }),
                None,
                false,
            )
            .unwrap_err();
        assert_eq!(error.code, "profileLocked");
        assert!(app.store.projects().is_empty());
        drop(locked);
    }

    #[test]
    fn a_failed_capture_restores_the_project_that_was_being_edited() {
        use std::os::windows::fs::OpenOptionsExt;

        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(temp.path().join("data")).unwrap();
        let first = user_folder_with(temp.path(), &["profile.sas7bcat", "regstry.sas7bitm"]);
        app.save_project(
            None,
            "P".into(),
            Some("note".into()),
            None,
            Some(ProfileChoice::SasUser {
                path: first.clone(),
                layers: Layers::DEFAULT,
            }),
            None,
            false,
        )
        .unwrap();
        let saved = app.store.projects()[0].clone();
        let original = saved.profile.clone().unwrap().revision;

        let second = temp.path().join("other").join("9.4");
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(second.join("profile.sas7bcat"), b"other").unwrap();
        std::fs::write(second.join("regstry.sas7bitm"), b"other").unwrap();
        let locked = std::fs::File::options()
            .read(true)
            .share_mode(0)
            .open(second.join("profile.sas7bcat"))
            .unwrap();

        let error = app
            .save_project(
                Some(saved.id),
                "Renamed".into(),
                None,
                None,
                Some(ProfileChoice::SasUser {
                    path: second.clone(),
                    layers: Layers::DEFAULT,
                }),
                None,
                false,
            )
            .unwrap_err();
        assert_eq!(error.code, "profileLocked");
        let stored = &app.store.projects()[0];
        assert_eq!(stored.name, "P", "the rename is rolled back too");
        assert_eq!(stored.profile.as_ref().unwrap().revision, original);
        drop(locked);
    }

    #[test]
    fn release_pages_are_built_from_a_validated_tag() {
        assert_eq!(
            release_url("v0.4.0").unwrap(),
            "https://github.com/wtwang1998/sas-launcher/releases/tag/v0.4.0"
        );
        assert_eq!(
            release_url("v0.4.0-rc.1").unwrap(),
            "https://github.com/wtwang1998/sas-launcher/releases/tag/v0.4.0-rc.1"
        );
        for rejected in ["", "v1/../2", "v1.0.0?x", "v1 0", "../../etc", "v1.0.0#"] {
            assert_eq!(
                release_url(rejected).unwrap_err().code,
                "releaseTagInvalid",
                "{rejected}"
            );
        }
    }
}
