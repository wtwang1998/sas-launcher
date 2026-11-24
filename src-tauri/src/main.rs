#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod autostart;
mod configuration;
mod diagnostics;
mod discovery;
mod issue;
mod language;
mod process;
mod profile;
mod sasuser;
mod session;
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
        match base {
            ConfigBase::Default => "默认配置".into(),
            ConfigBase::BuiltIn { code } => code.clone(),
            ConfigBase::User { id } => self
                .store
                .configuration(*id)
                .map(|config| config.name.clone())
                .unwrap_or_else(|| "默认配置".into()),
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
        };
        self.launch(resolved, config.name.clone(), Purpose::Setup { config: id })?;
        Ok(())
    }

    fn finish_config_setup(&mut self, id: Uuid, save: bool) -> Result<(), Issue> {
        self.sessions.finish_setup(id, save)?;
        // The session layer commits profile_revision directly; reload metadata.
        self.store.reload()
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
        let mut resolved = match self.resolve(project.config.as_ref()) {
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
        // A project-level choice owns the preference: either its own snapshot or
        // nothing at all. Only projects that predate the field keep following
        // their configuration.
        if project.profile.is_some() {
            resolved.profile = self.store.project_profile_source(&project);
        }
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
            let outcome = match self.resolve(project.config.as_ref()) {
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

fn with_app<T>(
    state: &AppState,
    action: impl FnOnce(&mut App) -> Result<T, Issue>,
) -> Result<T, Issue> {
    let mut app = state.lock().map_err(|_| Issue::new("stateUnavailable"))?;
    action(app.as_mut().map_err(|error| error.clone())?)
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

/// Hide the window in the tray; the first call also retires the tray hint.
#[tauri::command]
async fn hide_to_tray(handle: AppHandle, state: State<'_, AppState>) -> Result<View, Issue> {
    let view = with_app(&state, |app| {
        app.store
            .update(|settings| settings.tray_hint_seen = true)?;
        Ok(app.view())
    })?;
    if let Some(window) = handle.get_webview_window("main") {
        let _ = window.hide();
    }
    Ok(view)
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
    refresh_tray_menu(&handle, labels);
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
async fn save_project(
    state: State<'_, AppState>,
    id: Option<Uuid>,
    name: String,
    note: Option<String>,
    config: Option<ConfigRef>,
    profile: Option<ProfileChoice>,
) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.save_project(id, name, note, config, profile)?;
        Ok(app.view())
    })
}

impl App {
    /// Save a project and materialise the preference snapshot it asked for.
    ///
    /// The row is written first because the capture commits its revision
    /// through `projects.json`; a failed capture puts the row back instead of
    /// leaving a project that claims to inherit settings it never copied.
    fn save_project(
        &mut self,
        id: Option<Uuid>,
        name: String,
        note: Option<String>,
        config: Option<ConfigRef>,
        choice: Option<ProfileChoice>,
    ) -> Result<(), Issue> {
        let name = validate_name(&name)?;
        let note = validate_note(note)?;
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
            None | Some(ProfileChoice::Keep) => (
                previous
                    .as_ref()
                    .and_then(|project| project.profile.clone()),
                None,
            ),
            Some(ProfileChoice::Fresh { layers }) => (
                Some(ProjectProfile {
                    revision: None,
                    origin: Some(ProfileOrigin::Fresh),
                    layers,
                    fresh: true,
                }),
                None,
            ),
            Some(ProfileChoice::SasUser { path, layers }) => (
                Some(ProjectProfile {
                    revision: None,
                    origin: Some(ProfileOrigin::SasUser { path: path.clone() }),
                    layers,
                    fresh: false,
                }),
                Some(path),
            ),
            Some(ProfileChoice::ConfigSnapshot { config, layers }) => {
                let reference = ConfigRef::User { id: config };
                ensure_reference_exists(self, &reference)?;
                let source = self
                    .store
                    .configuration(config)
                    .and_then(|configuration| self.store.profile_revision_dir(configuration))
                    .filter(|directory| profile::has_catalog_files(directory))
                    .ok_or_else(|| Issue::new("profileEmpty"))?;
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

/// Native menu labels cannot use the TypeScript dictionary, so they are
/// resolved here from the same language preference.
fn tray_labels(app: &App) -> (&'static str, &'static str) {
    match app.store.settings.language.resolve(&app.system_language) {
        "zh-Hans" => ("显示窗口", "退出"),
        _ => ("Show window", "Quit"),
    }
}

fn tray_menu(app: &AppHandle, labels: (&str, &str)) -> tauri::Result<Menu<tauri::Wry>> {
    let show = MenuItem::with_id(app, "show", labels.0, true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", labels.1, true, None::<&str>)?;
    Menu::with_items(app, &[&show, &quit])
}

fn refresh_tray_menu(app: &AppHandle, labels: (&str, &str)) {
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
                .unwrap_or(("Show window", "Quit"));
            if let Some(icon) = app.default_window_icon().cloned() {
                let menu = tray_menu(app.handle(), labels)?;
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
            // Default: hide into the tray instead of shutting the app down.
            let close_to_tray = with_app(&window.state::<AppState>(), |app| {
                Ok(app.store.settings.close_to_tray)
            })
            .unwrap_or(true);
            if !close_to_tray {
                return;
            }
            api.prevent_close();
            let _ = window.app_handle().emit("close-requested", ());
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
            hide_to_tray,
            quit,
            clear_installation,
            save_project,
            delete_project,
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
            open_archive
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

    fn app_with_install(root: &Path, exe: &Path) -> App {
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
    fn deriving_from_user_config_copies_options_and_profile() {
        let temp = tempfile::tempdir().unwrap();
        let exe = install_fixture(
            temp.path(),
            &[("u8", "UTF-8", "zh_CN"), ("en", "wlatin1", "en_US")],
            "u8",
        );
        let mut app = app_with_install(temp.path(), &exe);
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
        if !missing.is_empty() {
            eprintln!(
                "Issue codes emitted by Rust but missing from src/issue-messages.json: {missing:?}"
            );
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
    fn a_new_project_inheriting_a_user_folder_captures_the_interface_layer_only() {
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
        assert!(!snapshot.join("regstry.sas7bitm").exists());
    }

    #[test]
    fn an_unreadable_source_leaves_no_project_behind() {
        use std::os::windows::fs::OpenOptionsExt;

        let temp = tempfile::tempdir().unwrap();
        let mut app = App::new(temp.path().join("data")).unwrap();
        let source = user_folder_with(temp.path(), &["profile.sas7bcat"]);
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
        let first = user_folder_with(temp.path(), &["profile.sas7bcat"]);
        app.save_project(
            None,
            "P".into(),
            Some("note".into()),
            None,
            Some(ProfileChoice::SasUser {
                path: first.clone(),
                layers: Layers::DEFAULT,
            }),
        )
        .unwrap();
        let saved = app.store.projects()[0].clone();
        let original = saved.profile.clone().unwrap().revision;

        let second = temp.path().join("other").join("9.4");
        std::fs::create_dir_all(&second).unwrap();
        std::fs::write(second.join("profile.sas7bcat"), b"other").unwrap();
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
            )
            .unwrap_err();
        assert_eq!(error.code, "profileLocked");
        let stored = &app.store.projects()[0];
        assert_eq!(stored.name, "P", "the rename is rolled back too");
        assert_eq!(stored.profile.as_ref().unwrap().revision, original);
        drop(locked);
    }
}
