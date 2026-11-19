#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod discovery;
mod issue;
mod language;
mod process;
mod profile;
mod session;
mod store;

use discovery::{ConfigKey, Installation, LaunchConfig, ProfileCandidate};
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
    sources: Vec<ProfileCandidate>,
    projects: Vec<Project>,
    templates: Vec<TemplateView>,
    sessions: Vec<SessionSummary>,
    scan_error: Option<Issue>,
    notices: Vec<Issue>,
    onboarding: Option<&'static str>,
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
        let store = Store::open(root)?;
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
                    .settings
                    .configurations
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
                self.installation = Some(installation);
                self.scan_error = None;
            }
            Err(error) => {
                self.installation = None;
                self.scan_error = Some(error);
            }
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

    fn managed_config(&self, id: Uuid) -> Result<ManagedConfig, Issue> {
        self.store
            .settings
            .configurations
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
    ) -> Result<Uuid, Issue> {
        self.refresh();
        let base_config = self.config(base)?;
        if base_config.custom_id.is_some() {
            return Err(Issue::new("configurationBaseMissing"));
        }
        let name = name.trim();
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
            number: self
                .store
                .settings
                .configurations
                .iter()
                .map(|item| item.number)
                .max()
                .unwrap_or_default()
                .saturating_add(1),
        });
        let replace_file = previous
            .as_ref()
            .is_none_or(|old| &old.base != base || !self.store.configuration_file_available(old));
        if previous.is_some() && replace_file {
            self.ensure_configuration_editable(&self.store.configuration_key(&config))?;
            config.revision = Uuid::new_v4();
        }
        config.base = base.clone();
        config.name = name.into();
        let id = config.id;
        self.store
            .update_configuration(config, previous.as_ref(), replace_file)?;
        self.refresh();
        Ok(id)
    }

    fn delete_configuration(&mut self, id: Uuid) -> Result<(), Issue> {
        let config = self.managed_config(id)?;
        let key = self.store.configuration_key(&config);
        if self
            .store
            .settings
            .projects
            .iter()
            .any(|project| project.key == key)
        {
            return Err(Issue::new("configurationInUse"));
        }
        self.ensure_configuration_editable(&key)?;
        self.store.delete_configuration(&config)?;
        self.refresh();
        Ok(())
    }

    fn language(&self) -> &'static str {
        self.store.settings.language.resolve(&self.system_language)
    }

    fn onboarding(&self) -> Option<&'static str> {
        let installation = self.installation.as_ref()?;
        if self.store.settings.onboarding_complete
            || installation
                .configs
                .iter()
                .all(|config| config.error.is_some())
        {
            return None;
        }
        if !self.store.has_user_data() && !installation.sources.is_empty() {
            return Some("first");
        }
        let default = installation.default_key.as_ref()?;
        if self.store.template_source(default).is_none()
            && installation
                .sources
                .iter()
                .any(|source| source.keys.contains(default))
        {
            return Some("upgrade");
        }
        None
    }

    fn view(&mut self) -> View {
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
            sources: installation
                .map(|item| item.sources.clone())
                .unwrap_or_default(),
            projects: self.store.settings.projects.clone(),
            templates,
            sessions: self.sessions.sweep(),
            scan_error: self.scan_error.clone(),
            notices: self.sessions.notices.clone(),
            onboarding: self.onboarding(),
            system_language: self.system_language.clone(),
            language_preference: self.store.settings.language,
            language: self.language(),
        }
    }

    fn launch(
        &mut self,
        key: &ConfigKey,
        name: String,
        id: Option<Uuid>,
    ) -> Result<SessionSummary, Issue> {
        let config = self.config(key)?;
        let template = self.store.template_path(key);
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
                match self.launch(&project.key, project.name.clone(), Some(project.id)) {
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

fn project_running(project: &Project, projects: &[Project], sessions: &[SessionSummary]) -> bool {
    let matches_project = |other: &Project| other.name == project.name && other.key == project.key;
    sessions
        .iter()
        .filter(|session| session.status == "running" && !session.template_setup)
        .any(|session| {
            session.project_id == Some(project.id)
                || (session.legacy_identity
                    && session.project_id.is_none()
                    && session.name == project.name
                    && session.key == project.key
                    && projects
                        .iter()
                        .filter(|other| matches_project(other))
                        .count()
                        == 1)
        })
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
        let installation = discovery::inspect(&path, app.documents.as_deref())?;
        app.store.update(|settings| settings.exe = Some(path))?;
        app.installation = Some(installation);
        app.scan_error = None;
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
async fn dismiss_onboarding(state: State<'_, AppState>) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.store
            .update(|settings| settings.onboarding_complete = true)?;
        Ok(app.view())
    })
}

#[tauri::command]
async fn save_project(
    state: State<'_, AppState>,
    id: Option<Uuid>,
    name: String,
    key: ConfigKey,
) -> Result<View, Issue> {
    with_app(&state, |app| {
        let name = validate_name(&name)?;
        app.config(&key)?;
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
        let project = Project { id, name, key };
        app.store.update(|settings| {
            if let Some(old) = settings.projects.iter_mut().find(|old| old.id == id) {
                *old = project;
            } else {
                settings.projects.push(project);
            }
        })?;
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
) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.save_configuration(id, &name, &base)?;
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
    onboarding: bool,
) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.config(&key)?;
        if onboarding && app.store.template_source(&key).is_some() {
            return Err(Issue::new("templateAlreadyReady"));
        }
        let directory = dunce::canonicalize(&directory)
            .map_err(|error| Issue::with_detail("sourceUnavailable", error))?;
        let source = TemplateSource::Imported {
            directory: directory.clone(),
        };
        app.store
            .install_template(key, &directory, source, onboarding)?;
        Ok(app.view())
    })
}

#[tauri::command]
async fn use_factory(
    state: State<'_, AppState>,
    key: ConfigKey,
    onboarding: bool,
) -> Result<View, Issue> {
    with_app(&state, |app| {
        app.config(&key)?;
        app.store.use_factory(key, onboarding)?;
        Ok(app.view())
    })
}

#[tauri::command]
async fn launch_project(
    state: State<'_, AppState>,
    key: ConfigKey,
    name: String,
    id: Option<Uuid>,
) -> Result<SessionSummary, Issue> {
    with_app(&state, |app| {
        app.refresh();
        if let Some(id) = id {
            let project = app
                .store
                .settings
                .projects
                .iter()
                .find(|project| project.id == id)
                .ok_or(Issue::new("projectMissing"))?
                .clone();
            return app.launch(&project.key, project.name, Some(id));
        }
        let name = temporary_project_name(&name, app.language())?;
        app.launch(&key, name, None)
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
                .install_template(key, &directory, TemplateSource::Customized, false)?;
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
            dismiss_onboarding,
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
    fn managed_configs_keep_project_identity_when_the_base_changes() {
        use std::fs;

        let temp = tempfile::tempdir().unwrap();
        let install = temp.path().join("install");
        fs::create_dir(&install).unwrap();
        let exe = install.join("sas.exe");
        fs::write(&exe, b"fixture").unwrap();
        for (code, encoding) in [("en", "wlatin1"), ("u8", "utf-8")] {
            let dir = install.join("nls").join(code);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("sasv9.cfg"), format!("-ENCODING {encoding}")).unwrap();
        }
        let mut app = App::new(temp.path().join("data"), None).unwrap();
        app.store
            .update(|settings| settings.exe = Some(exe.clone()))
            .unwrap();
        app.refresh();
        let configs = app.installation.as_ref().unwrap().configs.clone();
        let en = configs.iter().find(|config| config.code == "en").unwrap();
        let u8 = configs.iter().find(|config| config.code == "u8").unwrap();

        let id = app.save_configuration(None, "", &en.key).unwrap();
        let original = app.store.settings.configurations[0].clone();
        let key = app.store.configuration_key(&original);
        let before = app.config(&key).unwrap();
        assert_eq!(fs::read(before.cfg_path()).unwrap(), b"-ENCODING wlatin1");

        app.store
            .update(|settings| {
                settings.projects.push(Project {
                    id: Uuid::new_v4(),
                    name: "Bound".into(),
                    key: key.clone(),
                })
            })
            .unwrap();
        assert_eq!(
            app.delete_configuration(id).unwrap_err().code,
            "configurationInUse"
        );

        app.save_configuration(Some(id), "Renamed", &en.key)
            .unwrap();
        assert_eq!(app.config(&key).unwrap().cfg_path(), before.cfg_path());
        app.save_configuration(Some(id), "Renamed", &u8.key)
            .unwrap();
        let after = app.config(&key).unwrap();
        assert_eq!(fs::read(after.cfg_path()).unwrap(), b"-ENCODING utf-8");
        assert!(!before.cfg_path().exists());
        assert_eq!(app.store.settings.projects[0].key, key);

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
        assert_eq!(fs::read(&u8.key.cfg).unwrap(), b"-ENCODING utf-8");
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
            key: ConfigKey {
                exe: "sas.exe".into(),
                cfg: "sasv9.cfg".into(),
            },
        };
        let mut session = SessionSummary {
            id: Uuid::new_v4(),
            key: project.key.clone(),
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
        let duplicate = Project {
            id: Uuid::new_v4(),
            ..project.clone()
        };
        assert!(!project_running(
            &project,
            std::slice::from_ref(&project),
            std::slice::from_ref(&session)
        ));
        session.legacy_identity = true;
        assert!(project_running(
            &project,
            std::slice::from_ref(&project),
            std::slice::from_ref(&session)
        ));
        assert!(!project_running(
            &project,
            &[project.clone(), duplicate],
            std::slice::from_ref(&session)
        ));
        session.legacy_identity = false;
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
