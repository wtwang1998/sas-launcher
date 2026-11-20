#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod discovery;
mod issue;
mod language;
mod process;
mod profile;
mod session;
mod store;

use discovery::{ConfigKey, ConfigRef, Installation, LaunchConfig, ProfileCandidate};
use issue::Issue;
use language::LanguagePreference;
use serde::Serialize;
use session::{Purpose, SessionSummary, Sessions};
use std::{collections::HashSet, path::PathBuf, sync::Mutex, time::Duration};
use store::{ManagedConfig, Project, Store, TemplateSource};
use tauri::{Manager, State};
use uuid::Uuid;

struct App {
    store: Store,
    sessions: Sessions,
    documents: Option<PathBuf>,
    installation: Option<Installation>,
    scan_error: Option<Issue>,
    system_language: String,
}

type AppState = Mutex<Result<App, Issue>>;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct TemplateView {
    key: ConfigKey,
    source: TemplateSource,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct View {
    exe: Option<PathBuf>,
    configs: Vec<LaunchConfig>,
    default_key: Option<ConfigKey>,
    default_ref: Option<ConfigRef>,
    default_config: Option<LaunchConfig>,
    default_error: Option<Issue>,
    sources: Vec<ProfileCandidate>,
    projects: Vec<Project>,
    templates: Vec<TemplateView>,
    sessions: Vec<SessionSummary>,
    scan_error: Option<Issue>,
    notices: Vec<Issue>,
    system_language: String,
    language_preference: LanguagePreference,
    language: &'static str,
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
    fn new(root: PathBuf, documents: Option<PathBuf>) -> Result<Self, Issue> {
        let store = Store::open(root.join("v3"))?;
        let mut app = Self {
            sessions: Sessions::new(store.root.join("sessions")),
            store,
            documents,
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
            .and_then(|exe| discovery::inspect(&exe, self.documents.as_deref()));
        match result {
            Ok(mut installation) => {
                let exe = installation.configs[0].key.exe.clone();
                for managed in self
                    .store
                    .configurations()
                    .iter()
                    .filter(|item| item.base.exe == exe)
                {
                    let key = self.store.configuration_key(managed);
                    let base = installation
                        .configs
                        .iter()
                        .find(|config| config.key == managed.base);
                    let file = self.store.configuration_file(managed);
                    let code = base.map(|cfg| cfg.code.clone()).unwrap_or_default();
                    let mut config = discovery::describe(
                        ConfigKey {
                            exe: exe.clone(),
                            cfg: file.clone(),
                        },
                        code,
                        self.documents.as_deref(),
                    );
                    if !self.store.configuration_file_available(managed) {
                        config.error = Some(Issue::new("configurationFileMissing"));
                    } else if base.is_none_or(|base| base.error.is_some()) {
                        config.error = Some(Issue::new("configurationBaseMissing"));
                    }
                    config.custom_id = Some(managed.id);
                    config.custom_name = Some(managed.name.clone());
                    config.base = Some(managed.base.clone());
                    config.reference = ConfigRef::Custom { id: managed.id };
                    config.is_default = false;
                    config.file = Some(file);
                    config.key = key;
                    config.code = managed.number.to_string();
                    if let Some(directory) = &config.default_profile {
                        discovery::add_profile_candidate(
                            &mut installation.sources,
                            directory,
                            Some(&config.key),
                        );
                    }
                    installation.configs.push(config);
                }
                for config in &mut installation.configs {
                    config.is_default = config.custom_id.is_none()
                        && installation.default_key.as_ref() == Some(&config.key);
                }
                self.installation = Some(installation);
                self.scan_error = None;
            }
            Err(error) => {
                self.installation = None;
                self.scan_error = Some(error);
            }
        }
    }

    fn resolve_project_reference(
        &self,
        reference: Option<&ConfigRef>,
    ) -> Result<LaunchConfig, Issue> {
        match reference {
            Some(reference) => self.config_ref(reference),
            None => self.default_config(),
        }
    }

    fn config(&self, key: &ConfigKey) -> Result<LaunchConfig, Issue> {
        let config = self
            .installation
            .as_ref()
            .and_then(|installation| {
                installation
                    .configs
                    .iter()
                    .find(|config| &config.key == key)
            })
            .ok_or(Issue::new("configUnavailable"))?;
        if let Some(error) = &config.error {
            return Err(error.clone());
        }
        Ok(config.clone())
    }

    fn config_ref(&self, reference: &ConfigRef) -> Result<LaunchConfig, Issue> {
        if let ConfigRef::BuiltIn { exe, code } = reference {
            let config = self
                .installation
                .as_ref()
                .filter(|installation| installation.configs.iter().any(|item| &item.key.exe == exe))
                .and_then(|installation| {
                    installation
                        .configs
                        .iter()
                        .find(|item| item.code == *code && &item.key.exe == exe)
                })
                .ok_or(Issue::new("configUnavailable"))?;
            if let Some(error) = &config.error {
                return Err(error.clone());
            }
            return Ok(config.clone());
        }
        let config = self
            .installation
            .as_ref()
            .and_then(|installation| {
                installation
                    .configs
                    .iter()
                    .find(|config| &config.reference == reference)
            })
            .ok_or(Issue::new("configUnavailable"))?;
        if let Some(error) = &config.error {
            return Err(error.clone());
        }
        Ok(config.clone())
    }

    fn default_config(&self) -> Result<LaunchConfig, Issue> {
        let installation = self.installation.as_ref().ok_or_else(|| {
            self.scan_error
                .clone()
                .unwrap_or_else(|| Issue::new("configUnavailable"))
        })?;
        if let Some(error) = &installation.default_error {
            return Err(error.clone());
        }
        let key = installation
            .default_key
            .as_ref()
            .ok_or(Issue::new("defaultConfigPointerMissing"))?;
        let built_in = self.config(key)?;
        let mut config = discovery::describe_root_default(key, self.documents.as_deref())?;
        config.reference = built_in.reference;
        Ok(config)
    }

    fn config_for_reference(&self, reference: Option<&ConfigRef>) -> Result<LaunchConfig, Issue> {
        match reference {
            Some(reference) => self.config_ref(reference),
            None => self.default_config(),
        }
    }

    fn valid_option_pair(&self, encoding: &str, locale: &str) -> bool {
        self.canonical_option_pair(encoding, locale).is_some()
    }

    fn canonical_option_pair(&self, encoding: &str, locale: &str) -> Option<(String, String)> {
        let normalize = |value: &str| value.trim().replace('-', "_").to_ascii_lowercase();
        self.installation
            .as_ref()?
            .configs
            .iter()
            .find_map(|config| {
                if config.custom_id.is_some() || config.error.is_some() {
                    return None;
                }
                let canonical_encoding = config.encoding.as_deref()?;
                let canonical_locale = config.locale.as_deref()?;
                (normalize(canonical_encoding) == normalize(encoding)
                    && normalize(canonical_locale) == normalize(locale))
                .then(|| (canonical_encoding.to_owned(), canonical_locale.to_owned()))
            })
    }

    fn resolve_project_config(
        &mut self,
        reference: Option<&ConfigRef>,
        project_name: &str,
        encoding: Option<&str>,
        locale: Option<&str>,
    ) -> Result<Option<ConfigRef>, Issue> {
        let selected = self.config_for_reference(reference)?;
        let encoding = encoding.unwrap_or_else(|| selected.encoding.as_deref().unwrap_or_default());
        let locale = locale.unwrap_or_else(|| selected.locale.as_deref().unwrap_or_default());
        let same = |left: &str, right: &str| {
            left.trim()
                .replace('-', "_")
                .eq_ignore_ascii_case(&right.trim().replace('-', "_"))
        };
        if same(encoding, selected.encoding.as_deref().unwrap_or_default())
            && same(locale, selected.locale.as_deref().unwrap_or_default())
        {
            return Ok(reference.cloned());
        }
        let (encoding, locale) = self
            .canonical_option_pair(encoding, locale)
            .ok_or(Issue::new("invalidLocaleEncoding"))?;
        if !self.valid_option_pair(&encoding, &locale) {
            return Err(Issue::new("invalidLocaleEncoding"));
        }
        let base = selected.base.as_ref().unwrap_or(&selected.key).clone();
        let custom_name = if self.language() == "zh-Hans" {
            format!("{project_name} 配置")
        } else {
            format!("{project_name} configuration")
        };
        let id = self.save_configuration(None, &custom_name, &base, &encoding, &locale)?;
        Ok(Some(ConfigRef::Custom { id }))
    }

    fn managed_config(&self, id: Uuid) -> Result<ManagedConfig, Issue> {
        self.store
            .configurations()
            .iter()
            .find(|config| config.id == id)
            .cloned()
            .ok_or(Issue::new("managedConfigurationMissing"))
    }

    fn ensure_configuration_editable(&mut self, key: &ConfigKey) -> Result<(), Issue> {
        if self
            .sessions
            .sweep()
            .iter()
            .any(|session| &session.key == key && session.template_setup)
        {
            return Err(Issue::new("configurationSetupActive"));
        }
        Ok(())
    }

    fn save_configuration(
        &mut self,
        id: Option<Uuid>,
        name: &str,
        base: &ConfigKey,
        encoding: &str,
        locale: &str,
    ) -> Result<Uuid, Issue> {
        self.refresh();
        let base_config = self.config(base)?;
        if base_config.custom_id.is_some() {
            return Err(Issue::new("configurationBaseMissing"));
        }
        let (encoding, locale) = self
            .canonical_option_pair(encoding, locale)
            .ok_or(Issue::new("invalidLocaleEncoding"))?;
        let mut name = name.trim().to_owned();
        if name.chars().count() > 100 || name.chars().any(char::is_control) {
            return Err(Issue::new("invalidConfigurationName"));
        }
        let previous = id.map(|id| self.managed_config(id)).transpose()?;
        if let Some(previous) = &previous
            && previous.base.exe != base.exe
        {
            return Err(Issue::new("configurationBaseMissing"));
        }
        let mut config = previous.clone().unwrap_or_else(|| ManagedConfig {
            id: Uuid::new_v4(),
            name: String::new(),
            base: base.clone(),
            revision: Uuid::new_v4(),
            encoding: None,
            locale: None,
            number: self
                .store
                .configurations()
                .iter()
                .map(|item| item.number)
                .max()
                .unwrap_or_default()
                .saturating_add(1),
        });
        let replace_file = previous.as_ref().is_none_or(|old| {
            &old.base != base
                || old.encoding.as_deref() != Some(encoding.as_str())
                || old.locale.as_deref() != Some(locale.as_str())
                || !self.store.configuration_file_available(old)
        });
        if previous.is_some() && replace_file {
            self.ensure_configuration_editable(&self.store.configuration_key(&config))?;
            config.revision = Uuid::new_v4();
        }
        config.base = base.clone();
        config.encoding = Some(encoding.clone());
        config.locale = Some(locale.clone());
        if name.is_empty() {
            name = format!(
                "{} {}",
                if self.language() == "zh-Hans" {
                    "自定义配置"
                } else {
                    "Custom Configuration"
                },
                config.number
            );
        }
        config.name = name;
        let id = config.id;
        self.store
            .update_configuration(config, previous.as_ref(), replace_file)?;
        self.refresh();
        Ok(id)
    }

    fn delete_configuration(&mut self, id: Uuid) -> Result<(), Issue> {
        let config = self.managed_config(id)?;
        let key = self.store.configuration_key(&config);
        if configuration_in_use(id, &self.store.settings.projects) {
            return Err(Issue::new("configurationInUse"));
        }
        self.ensure_configuration_editable(&key)?;
        self.store.delete_configuration(&config)?;
        self.refresh();
        Ok(())
    }

    fn rollback_configuration_creation(&mut self, id: Uuid) -> Result<(), Issue> {
        self.store.rollback_configuration_creation(id)?;
        self.refresh();
        Ok(())
    }

    fn language(&self) -> &'static str {
        self.store.settings.language.resolve(&self.system_language)
    }

    fn view(&mut self) -> View {
        let resolved_default = self.default_config();
        let installation = self.installation.as_ref();
        let configs = installation
            .map(|item| item.configs.clone())
            .unwrap_or_default();
        let templates = configs
            .iter()
            .filter_map(|config| {
                let source = self.store.template_source(&config.key)?;
                Some(TemplateView {
                    key: config.key.clone(),
                    source,
                })
            })
            .collect();
        View {
            exe: self.exe(),
            configs,
            default_key: installation.and_then(|item| item.default_key.clone()),
            default_ref: installation.and_then(|item| {
                item.default_key
                    .as_ref()
                    .and_then(|key| item.configs.iter().find(|config| &config.key == key))
                    .map(|config| config.reference.clone())
            }),
            default_config: resolved_default.as_ref().ok().cloned(),
            default_error: installation
                .and_then(|item| item.default_error.clone())
                .or_else(|| resolved_default.err()),
            sources: installation
                .map(|item| item.sources.clone())
                .unwrap_or_default(),
            projects: self.store.settings.projects.clone(),
            templates,
            sessions: self.sessions.sweep(),
            scan_error: self.scan_error.clone(),
            notices: self.sessions.notices.clone(),
            system_language: self.system_language.clone(),
            language_preference: self.store.settings.language,
            language: self.language(),
        }
    }

    fn launch(
        &mut self,
        reference: Option<&ConfigRef>,
        name: String,
        id: Option<Uuid>,
    ) -> Result<SessionSummary, Issue> {
        self.refresh();
        let config = self.resolve_project_reference(reference)?;
        self.launch_resolved(config, name, id)
    }

    fn launch_resolved(
        &mut self,
        config: LaunchConfig,
        name: String,
        id: Option<Uuid>,
    ) -> Result<SessionSummary, Issue> {
        let template = self.store.template_path(&config.key);
        let cwd = self
            .documents
            .as_deref()
            .filter(|path| path.is_dir())
            .unwrap_or(&self.store.root);
        self.sessions.launch(
            &config,
            name,
            template.as_deref(),
            Purpose::Project(id),
            cwd,
        )
    }

    fn launch_batch(&mut self, ids: Vec<Uuid>) -> Vec<BatchResult> {
        self.refresh();
        let mut selected: HashSet<_> = ids.iter().copied().collect();
        let projects = self.store.settings.projects.clone();
        let running = self.sessions.sweep();
        let mut results = Vec::new();
        for project in &projects {
            if !selected.remove(&project.id) {
                continue;
            }
            let (status, reason) = if project_running(project, &projects, &running) {
                ("skipped", Some(Issue::new("batchRunning")))
            } else {
                let result = self
                    .resolve_project_reference(project.config.as_ref())
                    .and_then(|config| {
                        self.launch_resolved(config, project.name.clone(), Some(project.id))
                    });
                match result {
                    Ok(_) => ("started", None),
                    Err(reason) => ("failed", Some(reason)),
                }
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
}

fn project_running(project: &Project, _projects: &[Project], sessions: &[SessionSummary]) -> bool {
    sessions
        .iter()
        .filter(|session| session.status == "running" && !session.template_setup)
        .any(|session| session.project_id == Some(project.id))
}

fn configuration_in_use(config_id: Uuid, projects: &[Project]) -> bool {
    projects
        .iter()
        .any(|project| project.config == Some(ConfigRef::Custom { id: config_id }))
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

fn temporary_project_name(name: &str, language: &str) -> Result<String, Issue> {
    if !name.trim().is_empty() {
        return validate_name(name);
    }
    Ok(if language == "zh-Hans" {
        "临时项目"
    } else {
        "Temporary Project"
    }
    .into())
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
        discovery::inspect(&path, app.documents.as_deref())?;
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
async fn save_project(
    state: State<'_, AppState>,
    id: Option<Uuid>,
    name: String,
    note: Option<String>,
    config: Option<ConfigRef>,
    encoding: Option<String>,
    locale: Option<String>,
) -> Result<View, Issue> {
    with_app(&state, |app| {
        let name = validate_name(&name)?;
        let note = validate_note(note)?;
        let id = match id {
            Some(id)
                if app
                    .store
                    .settings
                    .projects
                    .iter()
                    .any(|project| project.id == id) =>
            {
                id
            }
            Some(_) => return Err(Issue::new("projectMissing")),
            None => Uuid::new_v4(),
        };
        let existing_config_ids: HashSet<_> = app
            .store
            .configurations()
            .iter()
            .map(|item| item.id)
            .collect();
        let config = app.resolve_project_config(
            config.as_ref(),
            &name,
            encoding.as_deref(),
            locale.as_deref(),
        )?;
        if let Some(ConfigRef::Custom { id }) = config.as_ref()
            && !app.store.configurations().iter().any(|item| item.id == *id)
        {
            return Err(Issue::new("managedConfigurationMissing"));
        }
        let project = Project {
            id,
            name,
            note,
            config: config.clone(),
        };
        if let Err(error) = app.store.update(|settings| {
            if let Some(old) = settings.projects.iter_mut().find(|old| old.id == id) {
                *old = project;
            } else {
                settings.projects.push(project);
            }
        }) {
            if let Some(ConfigRef::Custom { id: custom_id }) = config.as_ref()
                && !existing_config_ids.contains(custom_id)
            {
                let _ = app.rollback_configuration_creation(*custom_id);
            }
            return Err(error);
        }
        Ok(app.view())
    })
}

#[tauri::command]
async fn delete_project(state: State<'_, AppState>, id: Uuid) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.store
            .update(|settings| settings.projects.retain(|project| project.id != id))?;
        Ok(app.view())
    })
}

#[tauri::command]
async fn save_configuration(
    state: State<'_, AppState>,
    id: Option<Uuid>,
    name: String,
    base: ConfigKey,
    encoding: String,
    locale: String,
) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.save_configuration(id, &name, &base, &encoding, &locale)?;
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
async fn import_template(
    state: State<'_, AppState>,
    key: ConfigKey,
    directory: PathBuf,
) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.config(&key)?;
        let directory = dunce::canonicalize(&directory)
            .map_err(|error| Issue::with_detail("sourceUnavailable", error))?;
        let source = TemplateSource::Imported {
            directory: directory.clone(),
        };
        app.store.install_template(key, &directory, source)?;
        Ok(app.view())
    })
}

#[tauri::command]
async fn use_factory(state: State<'_, AppState>, key: ConfigKey) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.config(&key)?;
        app.store.use_factory(key)?;
        Ok(app.view())
    })
}

#[tauri::command]
async fn launch_project(
    state: State<'_, AppState>,
    config: Option<ConfigRef>,
    name: String,
    id: Option<Uuid>,
) -> Result<SessionSummary, Issue> {
    with_app(&state, |app| {
        if let Some(id) = id {
            let project = app
                .store
                .settings
                .projects
                .iter()
                .find(|project| project.id == id)
                .ok_or(Issue::new("projectMissing"))?
                .clone();
            return app.launch(project.config.as_ref(), project.name, Some(id));
        }
        let name = temporary_project_name(&name, app.language())?;
        app.launch(config.as_ref(), name, None)
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
async fn start_template_setup(state: State<'_, AppState>, key: ConfigKey) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.refresh();
        let config = app.config(&key)?;
        if app.sessions.sweep().iter().any(|session| {
            session.key == key && session.template_setup && session.status != "unknown"
        }) {
            return Err(Issue::new("setupAlreadyOpen"));
        }
        let baseline = app.store.template_path(&key);
        let title = if app.language() == "zh-Hans" {
            "SAS 个人设置"
        } else {
            "SAS Personal Settings"
        };
        let cwd = app
            .documents
            .as_deref()
            .filter(|path| path.is_dir())
            .unwrap_or(&app.store.root);
        app.sessions.launch(
            &config,
            title.into(),
            baseline.as_deref(),
            Purpose::Template,
            cwd,
        )?;
        Ok(app.view())
    })
}

#[tauri::command]
async fn finish_template_setup(
    state: State<'_, AppState>,
    id: Uuid,
    save: bool,
) -> Result<View, Issue> {
    with_app(&state, |app| {
        let (key, directory) = app.sessions.finish_setup(id, save)?;
        if save {
            app.store
                .install_template(key, &directory, TemplateSource::Customized)?;
        }
        app.sessions.retire_setup(id)?;
        Ok(app.view())
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
                Ok(root) => App::new(root, app.path().document_dir().ok()),
                Err(error) => Err(Issue::with_detail("dataPath", error)),
            });
            app.manage(state);
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                loop {
                    let state = handle.state::<AppState>();
                    let _ = with_app(&state, |app| {
                        app.sessions.sweep();
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
            save_project,
            delete_project,
            save_configuration,
            delete_configuration,
            import_template,
            use_factory,
            launch_project,
            launch_batch,
            start_template_setup,
            finish_template_setup
        ])
        .run(tauri::generate_context!())
        .expect("Could not run SAS Launcher");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn application_uses_fresh_versioned_data_without_migrating_legacy_state() {
        use std::fs;

        let temp = tempfile::tempdir().unwrap();
        let legacy_settings = temp.path().join("settings.json");
        let legacy_contents = br#"{"exe":"legacy.exe","projects":[{"id":"b7f77973-f9e3-41c1-a3be-5aa85ee2e1f9","name":"Legacy","key":{"exe":"legacy.exe","cfg":"legacy.cfg"}}]}"#;
        fs::write(&legacy_settings, legacy_contents).unwrap();
        let legacy_config = temp.path().join("configurations/legacy/sasv9.cfg");
        fs::create_dir_all(legacy_config.parent().unwrap()).unwrap();
        fs::write(&legacy_config, b"legacy config").unwrap();

        let version_two_settings = temp.path().join("v2/settings.json");
        fs::create_dir_all(version_two_settings.parent().unwrap()).unwrap();
        let version_two_contents = br#"{"projects":[{"id":"b7f77973-f9e3-41c1-a3be-5aa85ee2e1f9","name":"Version Two","key":{"exe":"old.exe","cfg":"old.cfg"}}]}"#;
        fs::write(&version_two_settings, version_two_contents).unwrap();

        let app = App::new(temp.path().to_path_buf(), None).unwrap();

        assert_eq!(app.store.root, temp.path().join("v3"));
        assert!(app.store.settings.projects.is_empty());
        assert_eq!(fs::read(&legacy_settings).unwrap(), legacy_contents);
        assert_eq!(fs::read(&legacy_config).unwrap(), b"legacy config");
        assert_eq!(
            fs::read(&version_two_settings).unwrap(),
            version_two_contents
        );
    }

    #[test]
    fn managed_configs_keep_project_identity_when_the_base_changes() {
        use std::fs;

        let temp = tempfile::tempdir().unwrap();
        let install = temp.path().join("install");
        fs::create_dir(&install).unwrap();
        let exe = install.join("sas.exe");
        fs::write(&exe, b"fixture").unwrap();
        for (code, encoding, locale) in [("en", "wlatin1", "en_US"), ("u8", "utf-8", "zh_CN")] {
            let dir = install.join("nls").join(code);
            fs::create_dir_all(&dir).unwrap();
            fs::write(
                dir.join("sasv9.cfg"),
                format!("-ENCODING {encoding}\n-LOCALE {locale}"),
            )
            .unwrap();
        }
        fs::write(
            install.join("sasv9.cfg"),
            r#"-CONFIG "!SASROOT\nls\en\sasv9.cfg""#,
        )
        .unwrap();
        let mut app = App::new(temp.path().join("data"), None).unwrap();
        app.store
            .update(|settings| settings.exe = Some(exe.clone()))
            .unwrap();
        app.refresh();
        let configs = app.installation.as_ref().unwrap().configs.clone();
        let en = configs.iter().find(|config| config.code == "en").unwrap();
        let u8 = configs.iter().find(|config| config.code == "u8").unwrap();

        let id = app
            .save_configuration(None, "", &en.key, "wlatin1", "en_US")
            .unwrap();
        let original = app.store.configurations()[0].clone();
        let key = app.store.configuration_key(&original);
        let before = app.config(&key).unwrap();
        let original_bytes = fs::read(before.cfg_path()).unwrap();
        assert!(String::from_utf8_lossy(&original_bytes).contains("-ENCODING wlatin1"));
        assert!(String::from_utf8_lossy(&original_bytes).contains("-LOCALE en_US"));

        app.store
            .update(|settings| {
                settings.projects.push(Project {
                    id: Uuid::new_v4(),
                    name: "Bound".into(),
                    note: Some("kept".into()),
                    config: Some(ConfigRef::Custom { id }),
                })
            })
            .unwrap();
        assert_eq!(
            app.delete_configuration(id).unwrap_err().code,
            "configurationInUse"
        );

        app.save_configuration(Some(id), "Renamed", &en.key, "wlatin1", "en_US")
            .unwrap();
        assert_eq!(app.config(&key).unwrap().cfg_path(), before.cfg_path());
        app.save_configuration(Some(id), "Renamed", &u8.key, "UTF-8", "zh_CN")
            .unwrap();
        let after = app.config(&key).unwrap();
        let after_contents =
            String::from_utf8_lossy(&fs::read(after.cfg_path()).unwrap()).into_owned();
        assert!(after_contents.contains("-ENCODING utf-8"));
        assert!(after_contents.contains("-LOCALE zh_CN"));
        assert!(!before.cfg_path().exists());
        assert_eq!(
            app.store.settings.projects[0].config,
            Some(ConfigRef::Custom { id })
        );
        assert_eq!(app.store.settings.projects[0].note.as_deref(), Some("kept"));

        let mut reopened = App::new(temp.path().join("data"), None).unwrap();
        assert_eq!(
            reopened.config(&key).unwrap().encoding.as_deref(),
            Some("utf-8")
        );
        reopened
            .store
            .update(|settings| settings.projects.clear())
            .unwrap();
        reopened.delete_configuration(id).unwrap();
        assert!(!after.cfg_path().exists());
        assert!(
            String::from_utf8_lossy(&fs::read(&u8.key.cfg).unwrap()).contains("-ENCODING utf-8")
        );
        assert!(String::from_utf8_lossy(&fs::read(&u8.key.cfg).unwrap()).contains("-LOCALE zh_CN"));
    }

    #[test]
    fn default_configuration_tracks_the_root_pointer_and_its_common_options() {
        use std::fs;

        let temp = tempfile::tempdir().unwrap();
        let install = temp.path().join("install");
        fs::create_dir(&install).unwrap();
        let exe = install.join("sas.exe");
        fs::write(&exe, b"fixture").unwrap();
        for (code, encoding, locale) in [("en", "wlatin1", "en_US"), ("u8", "utf-8", "zh_CN")] {
            let folder = install.join("nls").join(code);
            fs::create_dir_all(&folder).unwrap();
            fs::write(
                folder.join("sasv9.cfg"),
                format!("-ENCODING {encoding}\n-LOCALE {locale}"),
            )
            .unwrap();
        }
        let root_cfg = install.join("sasv9.cfg");
        fs::write(&root_cfg, "-CONFIG \"!SASROOT\\nls\\en\\sasv9.cfg\"").unwrap();

        let mut app = App::new(temp.path().join("data"), None).unwrap();
        app.store
            .update(|settings| settings.exe = Some(exe))
            .unwrap();
        app.refresh();
        let first = app.default_config().unwrap();
        assert_eq!(first.cfg_path(), root_cfg);
        assert_eq!(first.key.cfg.file_name().unwrap(), "sasv9.cfg");
        assert_eq!(first.encoding.as_deref(), Some("wlatin1"));
        assert_eq!(first.locale.as_deref(), Some("en_US"));

        fs::write(&root_cfg, "-CONFIG \"!SASROOT\\nls\\u8\\sasv9.cfg\"").unwrap();
        app.refresh();
        let second = app.default_config().unwrap();
        assert_eq!(second.cfg_path(), root_cfg);
        assert_eq!(second.encoding.as_deref(), Some("utf-8"));
        assert_eq!(second.locale.as_deref(), Some("zh_CN"));
        assert!(second.is_default);
    }

    #[test]
    fn changing_only_project_encoding_keeps_the_selected_locale() {
        use std::fs;

        let temp = tempfile::tempdir().unwrap();
        let install = temp.path().join("install");
        fs::create_dir_all(install.join("nls/aa1")).unwrap();
        fs::create_dir_all(install.join("nls/aa2")).unwrap();
        let exe = install.join("sas.exe");
        fs::write(&exe, b"fixture").unwrap();
        fs::write(
            install.join("nls/aa1/sasv9.cfg"),
            "-ENCODING wlatin1\n-LOCALE en_US",
        )
        .unwrap();
        fs::write(
            install.join("nls/aa2/sasv9.cfg"),
            "-ENCODING utf-8\n-LOCALE en_US",
        )
        .unwrap();
        fs::write(
            install.join("sasv9.cfg"),
            r#"-CONFIG "!SASROOT\nls\aa1\sasv9.cfg""#,
        )
        .unwrap();
        let mut app = App::new(temp.path().join("data"), None).unwrap();
        app.store
            .update(|settings| settings.exe = Some(exe))
            .unwrap();
        app.refresh();

        let reference = app
            .resolve_project_config(None, "Project", Some("UTF-8"), None)
            .unwrap();

        assert!(matches!(reference, Some(ConfigRef::Custom { .. })));
        let custom = app.config_ref(reference.as_ref().unwrap()).unwrap();
        assert_eq!(custom.encoding.as_deref(), Some("utf-8"));
        assert_eq!(custom.locale.as_deref(), Some("en_US"));
    }

    #[test]
    fn project_option_values_must_match_a_scanned_configuration_pair() {
        use std::fs;

        let temp = tempfile::tempdir().unwrap();
        let install = temp.path().join("install");
        fs::create_dir_all(install.join("nls/en")).unwrap();
        let exe = install.join("sas.exe");
        fs::write(&exe, b"fixture").unwrap();
        fs::write(
            install.join("nls/en/sasv9.cfg"),
            "-ENCODING wlatin1\n-LOCALE en_US",
        )
        .unwrap();
        fs::write(
            install.join("sasv9.cfg"),
            r#"-CONFIG "!SASROOT\nls\en\sasv9.cfg""#,
        )
        .unwrap();
        let mut app = App::new(temp.path().join("data"), None).unwrap();
        app.store
            .update(|settings| settings.exe = Some(exe))
            .unwrap();
        app.refresh();

        assert_eq!(
            app.resolve_project_config(None, "Project", Some("UTF-8"), Some("en_US"))
                .unwrap_err()
                .code,
            "invalidLocaleEncoding"
        );
    }

    #[test]
    fn projects_with_no_configuration_reference_resolve_the_current_install_default() {
        use std::fs;

        let temp = tempfile::tempdir().unwrap();
        let install = temp.path().join("install");
        fs::create_dir_all(install.join("nls/en")).unwrap();
        fs::create_dir_all(install.join("nls/u8")).unwrap();
        let exe = install.join("sas.exe");
        fs::write(&exe, b"fixture").unwrap();
        fs::write(
            install.join("nls/en/sasv9.cfg"),
            "-ENCODING wlatin1\n-LOCALE en_US",
        )
        .unwrap();
        fs::write(
            install.join("nls/u8/sasv9.cfg"),
            "-ENCODING utf-8\n-LOCALE zh_CN",
        )
        .unwrap();
        let root_cfg = install.join("sasv9.cfg");
        fs::write(&root_cfg, r#"-CONFIG "!SASROOT\nls\en\sasv9.cfg""#).unwrap();
        let mut app = App::new(temp.path().join("data"), None).unwrap();
        app.store
            .update(|settings| settings.exe = Some(exe))
            .unwrap();
        app.refresh();
        app.store
            .update(|settings| {
                settings.projects.push(Project {
                    id: Uuid::new_v4(),
                    name: "Default".into(),
                    note: None,
                    config: None,
                })
            })
            .unwrap();

        fs::write(&root_cfg, r#"-CONFIG "!SASROOT\nls\u8\sasv9.cfg""#).unwrap();
        app.refresh();
        let project = &app.store.settings.projects[0];
        let resolved = app
            .resolve_project_reference(project.config.as_ref())
            .unwrap();

        assert_eq!(resolved.cfg_path(), root_cfg);
        assert_eq!(resolved.encoding.as_deref(), Some("utf-8"));
        assert_eq!(resolved.locale.as_deref(), Some("zh_CN"));
    }

    #[test]
    fn project_option_changes_from_the_default_create_a_new_custom_reference() {
        use std::fs;

        let temp = tempfile::tempdir().unwrap();
        let install = temp.path().join("install");
        fs::create_dir_all(install.join("nls/en")).unwrap();
        fs::create_dir_all(install.join("nls/u8")).unwrap();
        let exe = install.join("sas.exe");
        fs::write(&exe, b"fixture").unwrap();
        fs::write(
            install.join("nls/en/sasv9.cfg"),
            "-ENCODING wlatin1\n-LOCALE en_US",
        )
        .unwrap();
        fs::write(
            install.join("nls/u8/sasv9.cfg"),
            "-ENCODING utf-8\n-LOCALE zh_CN",
        )
        .unwrap();
        fs::write(
            install.join("sasv9.cfg"),
            r#"-CONFIG "!SASROOT\nls\en\sasv9.cfg""#,
        )
        .unwrap();

        let mut app = App::new(temp.path().join("data"), None).unwrap();
        app.store
            .update(|settings| settings.exe = Some(exe))
            .unwrap();
        app.refresh();
        let reference = app
            .resolve_project_config(None, "Trial", Some("UTF-8"), Some("zh_CN"))
            .unwrap()
            .unwrap();

        assert!(matches!(reference, ConfigRef::Custom { .. }));
        let config = app.config_ref(&reference).unwrap();
        let contents = String::from_utf8_lossy(&fs::read(config.cfg_path()).unwrap()).into_owned();
        assert!(contents.contains("-ENCODING utf-8"));
        assert!(contents.contains("-LOCALE zh_CN"));
        assert!(contents.contains("-ENCODING wlatin1"));
        assert_eq!(app.store.configurations().len(), 1);
    }

    #[test]
    fn built_in_config_reference_tracks_installation_and_directory_code() {
        use std::fs;

        let temp = tempfile::tempdir().unwrap();
        let install = temp.path().join("install");
        fs::create_dir_all(install.join("nls/1d")).unwrap();
        fs::write(install.join("sas.exe"), b"fixture").unwrap();
        fs::write(
            install.join("nls/1d/sasv9.cfg"),
            "-ENCODING euc-cn\n-LOCALE zh_CN",
        )
        .unwrap();
        fs::write(
            install.join("sasv9.cfg"),
            r#"-CONFIG "!SASROOT\nls\1d\sasv9.cfg""#,
        )
        .unwrap();
        let mut app = App::new(temp.path().join("data"), None).unwrap();
        app.store
            .update(|settings| settings.exe = Some(install.join("sas.exe")))
            .unwrap();
        app.refresh();

        let reference = app.installation.as_ref().unwrap().configs[0]
            .reference
            .clone();
        let selected = app.config_ref(&reference).unwrap();
        assert_eq!(selected.cfg_path(), install.join("nls/1d/sasv9.cfg"));
        assert_eq!(
            reference,
            ConfigRef::BuiltIn {
                exe: install.join("sas.exe"),
                code: "1d".into()
            }
        );
    }

    #[test]
    fn built_in_reference_reads_the_installed_nls_file_after_it_changes() {
        use std::fs;

        let temp = tempfile::tempdir().unwrap();
        let install = temp.path().join("install");
        fs::create_dir_all(install.join("nls/1d")).unwrap();
        let exe = install.join("sas.exe");
        let nls_cfg = install.join("nls/1d/sasv9.cfg");
        fs::write(&exe, b"fixture").unwrap();
        fs::write(&nls_cfg, "-ENCODING euc-cn\n-LOCALE zh_CN").unwrap();
        fs::write(
            install.join("sasv9.cfg"),
            r#"-CONFIG "!SASROOT\nls\1d\sasv9.cfg""#,
        )
        .unwrap();
        let mut app = App::new(temp.path().join("data"), None).unwrap();
        app.store
            .update(|settings| settings.exe = Some(exe))
            .unwrap();
        app.refresh();
        let reference = app.installation.as_ref().unwrap().configs[0]
            .reference
            .clone();

        fs::write(&nls_cfg, "-ENCODING utf-8\n-LOCALE zh_CN").unwrap();
        app.refresh();
        let config = app.config_ref(&reference).unwrap();

        assert_eq!(config.cfg_path(), nls_cfg);
        assert_eq!(config.encoding.as_deref(), Some("utf-8"));
    }

    #[test]
    fn invalid_default_pointer_prevents_a_default_launch_resolution() {
        use std::fs;

        let temp = tempfile::tempdir().unwrap();
        let install = temp.path().join("install");
        fs::create_dir_all(install.join("nls/en")).unwrap();
        let exe = install.join("sas.exe");
        fs::write(&exe, b"fixture").unwrap();
        fs::write(
            install.join("nls/en/sasv9.cfg"),
            "-ENCODING wlatin1\n-LOCALE en_US",
        )
        .unwrap();
        fs::write(
            install.join("sasv9.cfg"),
            r#"-CONFIG "!SASROOT\nls\missing\sasv9.cfg""#,
        )
        .unwrap();

        let mut app = App::new(temp.path().join("data"), None).unwrap();
        app.store
            .update(|settings| settings.exe = Some(exe))
            .unwrap();
        app.refresh();

        assert_eq!(
            app.default_config().unwrap_err().code,
            "defaultConfigUnavailable"
        );
    }

    #[test]
    fn custom_configurations_are_protected_when_a_project_references_them() {
        let id = Uuid::new_v4();
        let project = Project {
            id: Uuid::new_v4(),
            name: "Notes".into(),
            note: None,
            config: Some(ConfigRef::Custom { id }),
        };
        assert!(configuration_in_use(id, std::slice::from_ref(&project)));
        assert!(!configuration_in_use(Uuid::new_v4(), &[project]));
    }

    #[test]
    fn temporary_projects_use_a_localized_name_when_blank() {
        assert_eq!(temporary_project_name("", "zh-Hans").unwrap(), "临时项目");
        assert_eq!(
            temporary_project_name("   ", "en").unwrap(),
            "Temporary Project"
        );
        assert_eq!(
            temporary_project_name("  Custom 项目 ", "en").unwrap(),
            "Custom 项目"
        );
        assert!(temporary_project_name("bad\nname", "en").is_err());
        assert!(validate_name("   ").is_err());
    }

    #[test]
    fn running_project_identity_handles_legacy_sessions_without_matching_new_quick_runs() {
        let project = Project {
            id: Uuid::new_v4(),
            name: "Report".into(),
            note: None,
            config: Some(ConfigRef::BuiltIn {
                exe: "sas.exe".into(),
                code: "en".into(),
            }),
        };
        let key = ConfigKey {
            exe: "sas.exe".into(),
            cfg: "sasv9.cfg".into(),
        };
        let mut session = SessionSummary {
            id: Uuid::new_v4(),
            key,
            name: project.name.clone(),
            title: "Report · UTF-8".into(),
            profile: "session".into(),
            args: vec![],
            template_setup: false,
            project_id: None,
            legacy_identity: false,
            status: "running",
            exit_code: None,
        };
        assert!(!project_running(
            &project,
            std::slice::from_ref(&project),
            std::slice::from_ref(&session)
        ));
        session.legacy_identity = true;
        assert!(!project_running(
            &project,
            std::slice::from_ref(&project),
            std::slice::from_ref(&session)
        ));
        session.project_id = Some(project.id);
        session.name = "Previous project name".into();
        assert!(project_running(
            &project,
            std::slice::from_ref(&project),
            std::slice::from_ref(&session)
        ));
        session.status = "cleanupPending";
        assert!(!project_running(
            &project,
            std::slice::from_ref(&project),
            &[session]
        ));
    }
}
