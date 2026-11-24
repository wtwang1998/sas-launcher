#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod configuration;
mod diagnostics;
mod discovery;
mod issue;
mod language;
mod process;
mod profile;
mod session;
mod store;
mod theme;
mod view;

use discovery::{BuiltIn, Installation};
use issue::Issue;
use language::LanguagePreference;
use serde::Serialize;
use session::{FailureRequest, LaunchRequest, Purpose, Record, Sessions, Stage, Status};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Command,
    sync::Mutex,
    time::Duration,
};
use store::{ConfigBase, ConfigRef, Project, Store};
use tauri::{AppHandle, Manager, State};
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
        self.sessions.sweep();
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
        let resolved = match self.resolve(project.config.as_ref()) {
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
async fn set_language(
    state: State<'_, AppState>,
    preference: LanguagePreference,
) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.store
            .update(|settings| settings.language = preference)?;
        Ok(app.view())
    })
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

#[tauri::command]
async fn save_project(
    state: State<'_, AppState>,
    id: Option<Uuid>,
    name: String,
    note: Option<String>,
    config: Option<ConfigRef>,
) -> Result<View, Issue> {
    with_app(&state, |app| {
        let name = validate_name(&name)?;
        let note = validate_note(note)?;
        let id = match id {
            Some(id) if app.store.projects().iter().any(|project| project.id == id) => id,
            Some(_) => return Err(Issue::new("projectMissing")),
            None => Uuid::new_v4(),
        };
        if let Some(reference) = config.as_ref() {
            ensure_reference_exists(app, reference)?;
        }
        app.store.save_project(Project {
            id,
            name,
            note,
            config,
        })?;
        Ok(app.view())
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

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.unminimize();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let state: AppState = Mutex::new(match app.path().app_local_data_dir() {
                Ok(root) => App::new(root),
                Err(error) => Err(Issue::with_detail("dataPath", error)),
            });
            app.manage(state);
            if let Some(window) = app.get_webview_window("main") {
                let theme = with_app(&app.state::<AppState>(), |app| Ok(app.store.settings.theme))
                    .unwrap_or_default();
                let _ = window.set_theme(theme.window_theme());
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
        .invoke_handler(tauri::generate_handler![
            snapshot,
            refresh_configs,
            select_installation,
            set_language,
            set_theme,
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
}
