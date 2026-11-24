use crate::{
    App,
    configuration::{self, PresetStatus},
    diagnostics,
    discovery::BuiltIn,
    issue::{FailureNote, Issue},
    language::LanguagePreference,
    session::{Purpose, Record, Stage, Status},
    store::{ConfigBase, ConfigRef, Project, UserConfig},
    theme::ThemePreference,
};
use serde::Serialize;
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigView {
    pub reference: ConfigRef,
    pub name: String,
    pub encoding: Option<String>,
    pub locale: Option<String>,
    pub ui_language: Option<String>,
    pub preset: PresetStatus,
    pub derived_from: Option<String>,
    pub has_profile: bool,
    pub error: Option<Issue>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectView {
    pub id: Uuid,
    pub name: String,
    pub note: Option<String>,
    pub config: Option<ConfigRef>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChainView {
    pub label: String,
    pub thin_cfg: Option<PathBuf>,
    pub factory_cfg: Option<PathBuf>,
    pub encoding: Option<String>,
    pub locale: Option<String>,
    pub sasuser: PathBuf,
    pub log: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionView {
    pub id: Uuid,
    pub name: String,
    pub kind: &'static str,
    pub config_id: Option<Uuid>,
    pub status: Status,
    pub exit_code: Option<u32>,
    pub started_at: i64,
    pub stage: Stage,
    pub argv: Vec<String>,
    pub chain: ChainView,
    pub sasuser_writable: bool,
    pub errors: u32,
    pub warnings: u32,
    /// Read-only preflight hints for this session's chain.
    pub hints: Vec<Issue>,
    /// Why the launch failed, when it never reached a process.
    pub failure: Option<FailureNote>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub exe: Option<PathBuf>,
    pub default_config: Option<ConfigView>,
    pub default_error: Option<Issue>,
    pub scan_error: Option<Issue>,
    pub configs: Vec<ConfigView>,
    pub projects: Vec<ProjectView>,
    pub sessions: Vec<SessionView>,
    pub archived: Option<PathBuf>,
    pub language_preference: LanguagePreference,
    pub language: &'static str,
    pub system_language: String,
    pub theme_preference: ThemePreference,
}

pub fn build_view(app: &App) -> View {
    let mut configs = Vec::new();
    for built_in in factory_configs(app) {
        configs.push(configuration::built_in_view(
            built_in,
            &app.store.settings.presets,
        ));
    }
    for config in app.store.configurations() {
        configs.push(user_view(app, config));
    }
    let (default_config, default_error) = default_row(app);
    let projects = app.store.projects().iter().map(project_view).collect();
    let sessions = app
        .sessions
        .cached()
        .iter()
        .map(|record| build_session_view(app, record))
        .collect();
    let language = app.store.settings.language.resolve(&app.system_language);
    View {
        exe: exe(app),
        default_config,
        default_error,
        scan_error: app.scan_error.clone(),
        configs,
        projects,
        sessions,
        archived: app.store.archived().map(Path::to_path_buf),
        language_preference: app.store.settings.language,
        language,
        system_language: app.system_language.clone(),
        theme_preference: app.store.settings.theme,
    }
}

fn exe(app: &App) -> Option<PathBuf> {
    app.installation
        .as_ref()
        .map(|installation| installation.exe.clone())
        .or_else(|| app.store.settings.exe.clone())
}

fn factory_configs(app: &App) -> &[BuiltIn] {
    app.installation
        .as_ref()
        .map(|installation| installation.configs.as_slice())
        .unwrap_or(&[])
}

fn default_row(app: &App) -> (Option<ConfigView>, Option<Issue>) {
    let Some(installation) = app.installation.as_ref() else {
        return (None, app.scan_error.clone());
    };
    if let Some(error) = &installation.default_error {
        return (None, Some(error.clone()));
    }
    let Some(code) = installation.default.clone() else {
        return (None, Some(Issue::new("defaultConfigPointerMissing")));
    };
    let Some(built_in) = installation
        .configs
        .iter()
        .find(|config| config.code == code)
    else {
        return (None, Some(Issue::new("defaultConfigUnavailable")));
    };
    (
        Some(configuration::built_in_view(
            built_in,
            &app.store.settings.presets,
        )),
        None,
    )
}

fn is_regular_file(path: &Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file())
}

fn project_view(project: &Project) -> ProjectView {
    ProjectView {
        id: project.id,
        name: project.name.clone(),
        note: project.note.clone(),
        config: project.config.clone(),
    }
}

fn user_view(app: &App, config: &UserConfig) -> ConfigView {
    let thin = app.store.thin_cfg_path(config);
    let mut error = None;
    if !is_regular_file(&thin) {
        error = Some(Issue::new("configurationFileMissing"));
    } else if !is_regular_file(&config.factory) {
        error = Some(Issue::new("configurationBaseMissing"));
    }
    let preset = if error.is_some() {
        PresetStatus::Unverified
    } else {
        configuration::preset_status(
            &app.store.settings.presets,
            factory_configs(app),
            &config.encoding,
            &config.locale,
        )
    };
    ConfigView {
        reference: ConfigRef::User { id: config.id },
        name: config.name.clone(),
        encoding: Some(config.encoding.clone()),
        locale: Some(config.locale.clone()),
        ui_language: ui_language_of(app, &config.base, 0),
        preset,
        derived_from: Some(base_label(app, &config.base)),
        has_profile: config.profile_revision.is_some(),
        error,
    }
}

fn base_label(app: &App, base: &ConfigBase) -> String {
    match base {
        ConfigBase::Default => configuration::DEFAULT_BASE_LABEL.into(),
        ConfigBase::BuiltIn { code } => code.clone(),
        ConfigBase::User { id } => app
            .store
            .configuration(*id)
            .map(|config| config.name.clone())
            .unwrap_or_else(|| configuration::DEFAULT_BASE_LABEL.into()),
    }
}

fn ui_language_of(app: &App, base: &ConfigBase, depth: u8) -> Option<String> {
    if depth > 8 {
        return None;
    }
    match base {
        ConfigBase::Default => default_row(app).0.and_then(|row| row.ui_language),
        ConfigBase::BuiltIn { code } => factory_configs(app)
            .iter()
            .find(|config| config.code == *code)
            .and_then(|config| config.ui_language.clone()),
        ConfigBase::User { id } => app
            .store
            .configuration(*id)
            .and_then(|config| ui_language_of(app, &config.base, depth + 1)),
    }
}

pub fn build_session_view(app: &App, record: &Record) -> SessionView {
    let log = app.sessions.log_path(record.id);
    let log = is_regular_file(&log).then_some(log);
    let (errors, warnings) = match log.as_deref() {
        Some(path) => diagnostics::scan_log(path),
        None => (0, 0),
    };
    let (kind, config_id) = match record.purpose {
        Purpose::Project { .. } => ("project", None),
        Purpose::Setup { config } => ("setup", Some(config)),
    };
    let chain = ChainView {
        label: record.name.clone(),
        thin_cfg: record.thin_cfg.clone(),
        factory_cfg: (!record.factory_cfg.as_os_str().is_empty())
            .then(|| record.factory_cfg.clone()),
        encoding: record.encoding.clone(),
        locale: record.locale.clone(),
        sasuser: record.sasuser.clone(),
        log,
    };
    let hints = diagnostics::self_check(&chain);
    SessionView {
        id: record.id,
        name: record.name.clone(),
        kind,
        config_id,
        status: record.status,
        exit_code: record.exit_code,
        started_at: record.started_at,
        stage: record.stage,
        argv: record.argv.clone(),
        chain,
        sasuser_writable: record.sasuser_writable,
        errors,
        warnings,
        hints,
        failure: record.failure.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::session::Sessions;
    use crate::store::Store;

    #[test]
    fn session_hints_expose_preflight_problems() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path().join("data")).unwrap();
        let sessions = Sessions::new(store.sessions_dir());
        let app = App {
            store,
            sessions,
            installation: None,
            scan_error: None,
            system_language: "en".into(),
        };
        let sasuser = temp.path().join("sasuser-file");
        std::fs::write(&sasuser, b"not a directory").unwrap();
        let record = Record {
            id: Uuid::new_v4(),
            name: "p".into(),
            title: "p".into(),
            purpose: Purpose::Project { id: None },
            stage: Stage::Spawn,
            status: Status::Exited,
            exit_code: Some(1),
            started_at: 0,
            argv: vec!["sas.exe".into()],
            sasuser,
            sasuser_writable: false,
            thin_cfg: None,
            factory_cfg: temp.path().join("missing-sasv9.cfg"),
            encoding: Some("utf-8".into()),
            locale: Some("zh_CN".into()),
            failure: None,
        };

        let view = build_session_view(&app, &record);
        let codes: Vec<&str> = view.hints.iter().map(|issue| issue.code).collect();
        assert!(codes.contains(&"configUnavailable"), "{codes:?}");
        assert!(codes.contains(&"profileNotWritable"), "{codes:?}");
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["hints"].as_array().map(Vec::len), Some(2));
    }

    #[test]
    fn empty_installation_yields_actionable_view() {
        let temp = tempfile::tempdir().unwrap();
        let store = Store::open(temp.path().join("data")).unwrap();
        let sessions = Sessions::new(store.sessions_dir());
        let app = App {
            store,
            sessions,
            installation: None,
            scan_error: Some(Issue::new("sasMissing")),
            system_language: "en".into(),
        };

        let view = build_view(&app);

        assert_eq!(view.exe, None);
        assert!(view.configs.is_empty());
        assert!(view.projects.is_empty());
        assert!(view.sessions.is_empty());
        assert_eq!(
            view.scan_error.as_ref().map(|error| error.code),
            Some("sasMissing")
        );
        assert!(view.default_config.is_none());
        assert_eq!(
            view.default_error.as_ref().map(|error| error.code),
            Some("sasMissing")
        );
        assert_eq!(view.language, "en");
        let json = serde_json::to_value(&view).unwrap();
        assert_eq!(json["exe"], serde_json::Value::Null);
        assert_eq!(json["scanError"]["code"], "sasMissing");
        assert_eq!(json["themePreference"], "system");
        assert_eq!(json["configs"], serde_json::Value::Array(Vec::new()));
    }
}
