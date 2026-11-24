use crate::{
    configuration,
    issue::Issue,
    language::LanguagePreference,
    profile::{Layers, copy_profile, copy_profile_layers, has_catalog_files, regular_directory},
    theme::ThemePreference,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File},
    io,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};
use tempfile::NamedTempFile;
use uuid::Uuid;

/// Legacy entries that are archived once into `<root>/_old-<YYYYMMDD>/`.
const LEGACY: &[&str] = &[
    "v3",
    "settings.json",
    "sessions",
    "configurations",
    "templates",
];

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConfigRef {
    BuiltIn { code: String },
    User { id: Uuid },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ConfigBase {
    Default,
    BuiltIn { code: String },
    User { id: Uuid },
}

/// Where a project's snapshot came from. Only ever displayed: it never decides
/// what gets seeded, so a stale origin cannot change a launch.
#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ProfileOrigin {
    SasUser { path: PathBuf },
    ConfigSnapshot { config: Uuid },
    Fresh,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProjectProfile {
    /// Snapshot revision under `projects/<id>/profile/`. Absent only for a
    /// project that picked a clean environment and has not run yet.
    #[serde(default)]
    pub revision: Option<Uuid>,
    #[serde(default)]
    pub origin: Option<ProfileOrigin>,
    #[serde(default)]
    pub layers: Layers,
    /// The user asked for a clean environment, so the configuration's own
    /// snapshot must never be used as a stand-in.
    #[serde(default)]
    pub fresh: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub config: Option<ConfigRef>,
    /// Absent on projects created before project-level preferences existed;
    /// those keep seeding from the configuration they point at.
    #[serde(default)]
    pub profile: Option<ProjectProfile>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct UserConfig {
    pub id: Uuid,
    pub name: String,
    pub base: ConfigBase,
    pub encoding: String,
    pub locale: String,
    pub revision: Uuid,
    #[serde(default)]
    pub profile_revision: Option<Uuid>,
    /// The factory cfg this thin cfg points at.
    #[serde(default)]
    pub factory: PathBuf,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Preset {
    pub encoding: String,
    pub locale: String,
    pub ok: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub exe: Option<PathBuf>,
    pub language: LanguagePreference,
    pub theme: ThemePreference,
    /// Closing the window keeps the app alive in the tray.
    pub close_to_tray: bool,
    /// Only meaningful while the login entry is registered.
    pub start_minimized: bool,
    pub presets: Vec<Preset>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            exe: None,
            language: LanguagePreference::default(),
            theme: ThemePreference::default(),
            close_to_tray: true,
            start_minimized: false,
            presets: Vec::new(),
        }
    }
}

pub struct Store {
    /// Application data root (`%LOCALAPPDATA%\com.saslauncher.desktop`).
    pub root: PathBuf,
    archived: Option<PathBuf>,
    archive_skipped: Vec<String>,
    pub settings: Settings,
    projects: Vec<Project>,
    configurations: Vec<UserConfig>,
}

pub struct CreateConfiguration {
    pub name: String,
    pub base: ConfigBase,
    pub factory: PathBuf,
    pub encoding: String,
    pub locale: String,
    /// Optional snapshot directory (the four catalog files) to copy into the
    /// new configuration.
    pub profile: Option<PathBuf>,
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> io::Result<T> {
    let file = File::open(path)?;
    Ok(serde_json::from_reader(io::BufReader::new(file))?)
}

/// Write JSON through a temp file in the same directory so a crash or a locked
/// target never leaves a truncated file behind.
pub fn write_json(path: &Path, value: &(impl Serialize + ?Sized)) -> io::Result<()> {
    let mut tmp = NamedTempFile::new_in(path.parent().unwrap())?;
    serde_json::to_writer_pretty(&mut tmp, value)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path)?;
    Ok(())
}

fn read_or_default<T: DeserializeOwned + Default>(path: &Path) -> Result<T, Issue> {
    match read_json(path) {
        Ok(value) => Ok(value),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(T::default()),
        Err(error) => Err(Issue::with_detail("settingsRead", error)),
    }
}

fn today_stamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or_default();
    let (year, month, day) = civil_from_days((seconds / 86_400) as i64);
    format!("{year:04}{month:02}{day:02}")
}

/// Howard Hinnant's days-from-civil inverse, so the archive folder does not
/// need a date-time dependency.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = (z - era * 146_097) as u64;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era as i64 + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_part = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_part + 2) / 5 + 1) as u32;
    let month = if month_part < 10 {
        month_part + 3
    } else {
        month_part - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

fn archive_legacy(root: &Path) -> Option<(PathBuf, Vec<String>)> {
    let present: Vec<&str> = LEGACY
        .iter()
        .copied()
        .filter(|name| root.join(name).exists())
        .collect();
    if present.is_empty() {
        return None;
    }
    // Never rename `root` itself: it also holds EBWebView/ (the WebView2 user
    // data directory) and renaming it while running breaks this process.
    let destination = root.join(format!("_old-{}", today_stamp()));
    if fs::create_dir_all(&destination).is_err() {
        return None;
    }
    let mut skipped = Vec::new();
    for name in present {
        let from = root.join(name);
        let to = destination.join(name);
        if to.exists() {
            // Already archived earlier today.
            continue;
        }
        if fs::rename(&from, &to).is_err() {
            skipped.push(name.to_owned());
        }
    }
    Some((destination, skipped))
}

impl Store {
    pub fn open(root: PathBuf) -> Result<Self, Issue> {
        fs::create_dir_all(&root).map_err(|error| Issue::with_detail("dataCreate", error))?;
        if !regular_directory(&root) {
            return Err(Issue::new("dataReparse"));
        }
        let data = root.join("v4");
        let mut archived = None;
        let mut archive_skipped = Vec::new();
        if !data.exists()
            && let Some((directory, skipped)) = archive_legacy(&root)
        {
            archived = Some(directory);
            archive_skipped = skipped;
        }
        for folder in ["configurations", "sessions"] {
            let path = data.join(folder);
            fs::create_dir_all(&path).map_err(|error| Issue::with_detail("dataCreate", error))?;
            if !regular_directory(&path) {
                return Err(Issue::new("dataReparse"));
            }
        }
        if !regular_directory(&data) {
            return Err(Issue::new("dataReparse"));
        }
        let settings = read_or_default(&data.join("settings.json"))?;
        let projects = read_or_default(&data.join("projects.json"))?;
        let configurations = read_or_default(&data.join("configurations.json"))?;
        Ok(Self {
            root,
            archived,
            archive_skipped,
            settings,
            projects,
            configurations,
        })
    }

    /// `<root>/v4`: all business data lives here.
    pub fn data_dir(&self) -> PathBuf {
        self.root.join("v4")
    }

    pub fn archived(&self) -> Option<&Path> {
        self.archived.as_deref()
    }

    #[allow(dead_code)]
    pub fn archive_skipped(&self) -> &[String] {
        &self.archive_skipped
    }

    pub fn sessions_dir(&self) -> PathBuf {
        self.data_dir().join("sessions")
    }

    pub fn configurations_dir(&self) -> PathBuf {
        self.data_dir().join("configurations")
    }

    pub fn configuration_dir(&self, id: Uuid) -> PathBuf {
        self.configurations_dir().join(id.to_string())
    }

    pub fn revision_dir(&self, id: Uuid, revision: Uuid) -> PathBuf {
        self.configuration_dir(id).join(revision.to_string())
    }

    pub fn thin_cfg_path(&self, config: &UserConfig) -> PathBuf {
        self.revision_dir(config.id, config.revision)
            .join("sasv9.cfg")
    }

    pub fn profile_dir(&self, id: Uuid, revision: Uuid) -> PathBuf {
        self.configuration_dir(id)
            .join("profile")
            .join(revision.to_string())
    }

    pub fn profile_revision_dir(&self, config: &UserConfig) -> Option<PathBuf> {
        config
            .profile_revision
            .map(|revision| self.profile_dir(config.id, revision))
    }

    pub fn projects_dir(&self) -> PathBuf {
        self.data_dir().join("projects")
    }

    pub fn project_dir(&self, id: Uuid) -> PathBuf {
        self.projects_dir().join(id.to_string())
    }

    pub fn project_profile_root(&self, id: Uuid) -> PathBuf {
        self.project_dir(id).join("profile")
    }

    pub fn project_profile_dir(&self, id: Uuid, revision: Uuid) -> PathBuf {
        self.project_profile_root(id).join(revision.to_string())
    }

    /// The snapshot directory that should seed this project's next session.
    /// A project-level choice always wins: an empty answer means "seed
    /// nothing", never "fall back to the configuration".
    pub fn project_profile_source(&self, project: &Project) -> Option<PathBuf> {
        let revision = project.profile.as_ref()?.revision?;
        let directory = self.project_profile_dir(project.id, revision);
        has_catalog_files(&directory).then_some(directory)
    }

    pub fn configurations(&self) -> &[UserConfig] {
        &self.configurations
    }

    pub fn configuration(&self, id: Uuid) -> Option<&UserConfig> {
        self.configurations.iter().find(|config| config.id == id)
    }

    pub fn projects(&self) -> &[Project] {
        &self.projects
    }

    fn settings_path(&self) -> PathBuf {
        self.data_dir().join("settings.json")
    }

    fn projects_path(&self) -> PathBuf {
        self.data_dir().join("projects.json")
    }

    fn configurations_path(&self) -> PathBuf {
        self.data_dir().join("configurations.json")
    }

    pub fn reload(&mut self) -> Result<(), Issue> {
        self.settings = read_or_default(&self.settings_path())?;
        self.projects = read_or_default(&self.projects_path())?;
        self.configurations = read_or_default(&self.configurations_path())?;
        Ok(())
    }

    pub fn update(&mut self, edit: impl FnOnce(&mut Settings)) -> Result<(), Issue> {
        let mut next = self.settings.clone();
        edit(&mut next);
        write_json(&self.settings_path(), &next)
            .map_err(|error| Issue::with_detail("settingsWrite", error))?;
        self.settings = next;
        Ok(())
    }

    pub fn save_project(&mut self, project: Project) -> Result<(), Issue> {
        let mut next = self.projects.clone();
        match next.iter_mut().find(|item| item.id == project.id) {
            Some(existing) => *existing = project,
            None => next.push(project),
        }
        write_json(&self.projects_path(), &next)
            .map_err(|error| Issue::with_detail("settingsWrite", error))?;
        self.projects = next;
        Ok(())
    }

    pub fn delete_project(&mut self, id: Uuid) -> Result<(), Issue> {
        let mut next = self.projects.clone();
        next.retain(|project| project.id != id);
        write_json(&self.projects_path(), &next)
            .map_err(|error| Issue::with_detail("settingsWrite", error))?;
        self.projects = next;
        // Best effort: the metadata is already gone, and a leftover snapshot
        // only wastes disk space.
        let _ = fs::remove_dir_all(self.project_dir(id));
        Ok(())
    }

    fn write_configurations(&self, next: &[UserConfig]) -> Result<(), Issue> {
        write_json(&self.configurations_path(), next)
            .map_err(|error| Issue::with_detail("settingsWrite", error))
    }

    fn write_thin(
        &self,
        id: Uuid,
        revision: Uuid,
        request: &CreateConfiguration,
    ) -> Result<(), Issue> {
        let directory = self.revision_dir(id, revision);
        fs::create_dir_all(&directory).map_err(|error| Issue::with_detail("dataCreate", error))?;
        if !regular_directory(&directory) {
            return Err(Issue::new("dataReparse"));
        }
        configuration::write_thin_cfg(
            &directory.join("sasv9.cfg"),
            &request.factory,
            &request.encoding,
            &request.locale,
        )
    }

    /// Create the thin cfg directory tree and commit the metadata. Any failure
    /// removes the partially created folder so no half-written configuration
    /// is left behind.
    pub fn create_configuration(&mut self, request: CreateConfiguration) -> Result<Uuid, Issue> {
        let id = Uuid::new_v4();
        let revision = Uuid::new_v4();
        let directory = self.configuration_dir(id);
        let result = (|| -> Result<(), Issue> {
            self.write_thin(id, revision, &request)?;
            let mut profile_revision = None;
            if let Some(source) = request.profile.as_deref()
                && crate::profile::has_catalog_files(source)
            {
                let profile_parent = directory.join("profile");
                fs::create_dir_all(&profile_parent)
                    .map_err(|error| Issue::with_detail("profileCreate", error))?;
                let staged = Uuid::new_v4();
                let staging = profile_parent.join(format!("tmp-{staged}"));
                fs::create_dir(&staging)
                    .map_err(|error| Issue::with_detail("profileCreate", error))?;
                if let Err(error) = copy_profile(source, &staging) {
                    let _ = fs::remove_dir_all(&staging);
                    return Err(error);
                }
                let target = profile_parent.join(staged.to_string());
                fs::rename(&staging, &target)
                    .map_err(|error| Issue::with_detail("profileCopy", error))?;
                profile_revision = Some(staged);
            }
            let config = UserConfig {
                id,
                name: request.name.clone(),
                base: request.base.clone(),
                encoding: request.encoding.trim().to_owned(),
                locale: request.locale.trim().to_owned(),
                revision,
                profile_revision,
                factory: request.factory.clone(),
            };
            let mut next = self.configurations.clone();
            next.push(config);
            self.write_configurations(&next)?;
            self.configurations = next;
            Ok(())
        })();
        match result {
            Ok(()) => Ok(id),
            Err(error) => {
                let _ = fs::remove_dir_all(&directory);
                Err(error)
            }
        }
    }

    /// Rotate the thin cfg revision and then commit the metadata. A metadata
    /// failure keeps the previous revision on disk and in configurations.json.
    pub fn update_configuration(
        &mut self,
        id: Uuid,
        encoding: &str,
        locale: &str,
    ) -> Result<(), Issue> {
        let index = self
            .configurations
            .iter()
            .position(|config| config.id == id)
            .ok_or_else(|| Issue::new("managedConfigurationMissing"))?;
        let current = self.configurations[index].clone();
        let revision = Uuid::new_v4();
        let factory = current.factory.clone();
        let request = CreateConfiguration {
            name: current.name.clone(),
            base: current.base.clone(),
            factory,
            encoding: encoding.trim().to_owned(),
            locale: locale.trim().to_owned(),
            profile: None,
        };
        if let Err(error) = self.write_thin(id, revision, &request) {
            let _ = fs::remove_dir_all(self.revision_dir(id, revision));
            return Err(error);
        }
        let mut next = self.configurations.clone();
        next[index].revision = revision;
        next[index].encoding = request.encoding;
        next[index].locale = request.locale;
        if let Err(error) = self.write_configurations(&next) {
            let _ = fs::remove_dir_all(self.revision_dir(id, revision));
            return Err(error);
        }
        self.configurations = next;
        let previous = self.revision_dir(id, current.revision);
        if previous != self.revision_dir(id, revision) {
            let _ = fs::remove_dir_all(previous);
        }
        Ok(())
    }

    pub fn delete_configuration(&mut self, id: Uuid) -> Result<(), Issue> {
        if self.configuration(id).is_none() {
            return Err(Issue::new("managedConfigurationMissing"));
        }
        let mut next = self.configurations.clone();
        next.retain(|config| config.id != id);
        self.write_configurations(&next)?;
        self.configurations = next;
        // Best effort: the metadata is already gone.
        let _ = fs::remove_dir_all(self.configuration_dir(id));
        Ok(())
    }
}

/// Persist a new profile revision without going through a `Store` handle. The
/// session layer owns the snapshot copy and only needs this final metadata
/// commit; the caller reloads the store afterwards.
pub fn set_profile_revision_on_disk(
    data_dir: &Path,
    id: Uuid,
    revision: Uuid,
) -> Result<(), Issue> {
    let path = data_dir.join("configurations.json");
    let mut configurations: Vec<UserConfig> = match read_json(&path) {
        Ok(configurations) => configurations,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Err(Issue::new("managedConfigurationMissing"));
        }
        Err(error) => return Err(Issue::with_detail("settingsRead", error)),
    };
    let entry = configurations
        .iter_mut()
        .find(|config| config.id == id)
        .ok_or_else(|| Issue::new("managedConfigurationMissing"))?;
    entry.profile_revision = Some(revision);
    write_json(&path, &configurations).map_err(|error| Issue::with_detail("settingsWrite", error))
}

/// How many project snapshots are kept: the current one plus what it replaced.
const KEEP_PROJECT_PROFILES: usize = 2;

fn read_projects(data_dir: &Path) -> Result<Vec<Project>, Issue> {
    let path = data_dir.join("projects.json");
    match read_json(&path) {
        Ok(projects) => Ok(projects),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(error) => Err(Issue::with_detail("settingsRead", error)),
    }
}

fn project_profile_root(data_dir: &Path, id: Uuid) -> PathBuf {
    data_dir
        .join("projects")
        .join(id.to_string())
        .join("profile")
}

/// The layers a project currently asks for, read straight from disk so the
/// session layer can honour them without holding a `Store`. `None` means the
/// project predates project-level preferences and still follows its
/// configuration.
pub fn project_layers(data_dir: &Path, id: Uuid) -> Option<Layers> {
    read_projects(data_dir).ok().and_then(|projects| {
        projects
            .into_iter()
            .find(|project| project.id == id)
            .and_then(|project| project.profile)
            .map(|profile| profile.layers)
    })
}

/// Copy `source` into a fresh revision under `projects/<id>/profile/`, then
/// commit the revision and the effective layers to `projects.json`.
///
/// A failed copy leaves the previous revision and the metadata untouched; the
/// caller keeps the source folder so the next sweep can retry.
pub fn capture_project_profile(
    data_dir: &Path,
    project_id: Uuid,
    source: &Path,
    layers: Option<Layers>,
) -> Result<Uuid, Issue> {
    let layers = layers
        .or_else(|| project_layers(data_dir, project_id))
        .unwrap_or(Layers::DEFAULT);
    let root = project_profile_root(data_dir, project_id);
    fs::create_dir_all(&root).map_err(|error| Issue::with_detail("profileCreate", error))?;
    if !regular_directory(&root) {
        return Err(Issue::new("dataReparse"));
    }
    let revision = Uuid::new_v4();
    let staging = root.join(format!("tmp-{revision}"));
    fs::create_dir(&staging).map_err(|error| Issue::with_detail("profileCreate", error))?;
    if let Err(error) = copy_profile_layers(source, &staging, layers) {
        let _ = fs::remove_dir_all(&staging);
        return Err(error);
    }
    let target = root.join(revision.to_string());
    if let Err(error) = fs::rename(&staging, &target) {
        let _ = fs::remove_dir_all(&staging);
        return Err(Issue::with_detail("profileCopy", error));
    }
    if let Err(error) = commit_project_profile(data_dir, project_id, revision, layers) {
        let _ = fs::remove_dir_all(&target);
        return Err(error);
    }
    prune_project_profiles(&root);
    Ok(revision)
}

fn commit_project_profile(
    data_dir: &Path,
    project_id: Uuid,
    revision: Uuid,
    layers: Layers,
) -> Result<(), Issue> {
    let path = data_dir.join("projects.json");
    let mut projects = read_projects(data_dir)?;
    let entry = projects
        .iter_mut()
        .find(|project| project.id == project_id)
        .ok_or_else(|| Issue::new("projectMissing"))?;
    let origin = entry.profile.take().and_then(|profile| profile.origin);
    entry.profile = Some(ProjectProfile {
        revision: Some(revision),
        origin,
        layers,
        fresh: false,
    });
    write_json(&path, &projects).map_err(|error| Issue::with_detail("settingsWrite", error))
}

/// Drop all but the newest [`KEEP_PROJECT_PROFILES`] revisions. Staging folders
/// are left alone: deleting one mid-copy would lose the only copy.
fn prune_project_profiles(root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut revisions: Vec<(SystemTime, PathBuf)> = entries
        .flatten()
        .filter(|entry| Uuid::parse_str(&entry.file_name().to_string_lossy()).is_ok())
        .filter_map(|entry| {
            let modified = entry.metadata().ok()?.modified().ok()?;
            Some((modified, entry.path()))
        })
        .collect();
    if revisions.len() <= KEEP_PROJECT_PROFILES {
        return;
    }
    revisions.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    for (_, path) in revisions.into_iter().skip(KEEP_PROJECT_PROFILES) {
        let _ = fs::remove_dir_all(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn count_archives(root: &Path) -> usize {
        fs::read_dir(root)
            .unwrap()
            .flatten()
            .filter(|entry| entry.file_name().to_string_lossy().starts_with("_old-"))
            .count()
    }

    #[test]
    fn settings_written_before_the_theme_existed_keep_the_new_defaults() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(temp.path().join("data")).unwrap();
        assert_eq!(store.settings.theme, ThemePreference::System);

        fs::write(
            store.data_dir().join("settings.json"),
            br#"{"language":"en"}"#,
        )
        .unwrap();
        store.reload().unwrap();
        assert!(matches!(store.settings.language, LanguagePreference::En));
        assert_eq!(store.settings.theme, ThemePreference::System);
        // A missing close-to-tray key must default to "keep running in the tray".
        assert!(store.settings.close_to_tray);
        assert!(!store.settings.start_minimized);

        store
            .update(|settings| settings.theme = ThemePreference::Dark)
            .unwrap();
        store.reload().unwrap();
        assert_eq!(store.settings.theme, ThemePreference::Dark);
        assert!(store.settings.close_to_tray);

        store
            .update(|settings| settings.close_to_tray = false)
            .unwrap();
        store.reload().unwrap();
        assert!(!store.settings.close_to_tray);

        store
            .update(|settings| settings.start_minimized = true)
            .unwrap();
        store.reload().unwrap();
        assert!(store.settings.start_minimized);

        // Keys this build retired are ignored, not rejected.
        fs::write(
            store.data_dir().join("settings.json"),
            br#"{"close_to_tray":false,"tray_hint_seen":true}"#,
        )
        .unwrap();
        store.reload().unwrap();
        assert!(!store.settings.close_to_tray);
    }

    #[test]
    fn archives_legacy_entries_once_and_keeps_webview_profile() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("data");
        fs::create_dir_all(root.join("v3")).unwrap();
        fs::write(root.join("v3/settings.json"), br#"{"exe":"legacy.exe"}"#).unwrap();
        fs::write(root.join("v3/configurations.json"), b"[]").unwrap();
        fs::write(root.join("settings.json"), br#"{"projects":[]}"#).unwrap();
        fs::create_dir_all(root.join("EBWebView/Default")).unwrap();
        fs::write(root.join("EBWebView/Default/marker"), b"keep").unwrap();

        let store = Store::open(root.clone()).unwrap();

        assert!(root.join("v4").is_dir());
        let archived = store.archived().expect("an archive folder");
        let name = archived.file_name().unwrap().to_string_lossy().into_owned();
        assert!(name.starts_with("_old-"), "{name}");
        let stamp = &name["_old-".len()..];
        assert_eq!(stamp.len(), 8, "{name}");
        assert!(stamp.chars().all(|ch| ch.is_ascii_digit()), "{name}");
        assert!(archived.join("v3/settings.json").is_file());
        assert!(archived.join("settings.json").is_file());
        // Never move the WebView2 profile, and never rename the root itself.
        assert_eq!(
            fs::read(root.join("EBWebView/Default/marker")).unwrap(),
            b"keep"
        );
        assert!(!root.join("settings.json").exists());

        Store::open(root.clone()).unwrap();
        assert_eq!(count_archives(&root), 1);
    }

    #[test]
    fn update_rotates_revision_and_keeps_previous_file_on_failure() {
        use std::os::windows::fs::OpenOptionsExt;

        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("data");
        let factory = temp.path().join("factory").join("sasv9.cfg");
        fs::create_dir_all(factory.parent().unwrap()).unwrap();
        fs::write(&factory, "-ENCODING wlatin1\n-LOCALE en_US\n").unwrap();
        let mut store = Store::open(root.clone()).unwrap();

        let id = store
            .create_configuration(CreateConfiguration {
                name: "Mine".into(),
                base: ConfigBase::Default,
                factory: factory.clone(),
                encoding: "wlatin1".into(),
                locale: "en_US".into(),
                profile: None,
            })
            .unwrap();
        let first = store.configuration(id).unwrap().clone();
        let first_file = store.thin_cfg_path(&first);
        let first_revision = first.revision;
        let first_contents = fs::read(&first_file).unwrap();

        let locked = File::options()
            .read(true)
            .share_mode(1)
            .open(root.join("v4/configurations.json"))
            .unwrap();
        assert!(store.update_configuration(id, "utf-8", "zh_CN").is_err());
        drop(locked);

        let reloaded = Store::open(root).unwrap();
        let after = reloaded.configuration(id).unwrap();
        assert_eq!(after.revision, first_revision);
        assert_eq!(after.encoding, "wlatin1");
        assert_eq!(fs::read(&first_file).unwrap(), first_contents);
        // The failed revision directory must not be left behind.
        let revisions = fs::read_dir(reloaded.revision_dir(id, first_revision).parent().unwrap())
            .unwrap()
            .flatten()
            .count();
        assert_eq!(revisions, 1);

        store.update_configuration(id, "utf-8", "zh_CN").unwrap();
        let rotated = store.configuration(id).unwrap();
        assert_ne!(rotated.revision, first_revision);
        assert!(!first_file.exists());
        let contents =
            String::from_utf8_lossy(&fs::read(store.thin_cfg_path(rotated)).unwrap()).into_owned();
        assert!(contents.contains("-ENCODING utf-8"));
    }

    fn seeded_project(data: &Path, id: Uuid) {
        write_json(
            &data.join("projects.json"),
            &vec![Project {
                id,
                name: "Project".into(),
                note: None,
                config: None,
                profile: Some(ProjectProfile {
                    revision: None,
                    origin: Some(ProfileOrigin::SasUser {
                        path: PathBuf::from(r"C:\Users\me\Documents\My SAS Files\9.4"),
                    }),
                    layers: Layers::DEFAULT,
                    fresh: false,
                }),
            }],
        )
        .unwrap();
    }

    fn source_with_every_catalog(temp: &Path) -> PathBuf {
        let source = temp.join("sasuser");
        fs::create_dir(&source).unwrap();
        for name in crate::profile::FILES {
            fs::write(source.join(name), name.as_bytes()).unwrap();
        }
        source
    }

    fn revision_names(root: &Path) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(root)
            .unwrap()
            .flatten()
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn capturing_a_project_profile_commits_layers_and_keeps_two_revisions() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("v4");
        fs::create_dir_all(&data).unwrap();
        let id = Uuid::new_v4();
        seeded_project(&data, id);
        let source = source_with_every_catalog(temp.path());

        let first = capture_project_profile(&data, id, &source, None).unwrap();
        let stored: Vec<Project> = read_json(&data.join("projects.json")).unwrap();
        let profile = stored[0].profile.as_ref().unwrap();
        assert_eq!(profile.revision, Some(first));
        assert_eq!(profile.layers, Layers::DEFAULT);
        assert!(
            matches!(profile.origin, Some(ProfileOrigin::SasUser { .. })),
            "the origin survives a capture"
        );
        let snapshot = project_profile_root(&data, id).join(first.to_string());
        assert!(snapshot.join("profile.sas7bcat").is_file());
        assert!(
            !snapshot.join("regstry.sas7bitm").exists(),
            "the environment layer stays out by default"
        );

        let second = capture_project_profile(&data, id, &source, Some(Layers::ALL)).unwrap();
        let third = capture_project_profile(&data, id, &source, Some(Layers::ALL)).unwrap();
        let kept = revision_names(&project_profile_root(&data, id));
        assert_eq!(kept.len(), KEEP_PROJECT_PROFILES);
        assert!(kept.contains(&third.to_string()));
        assert!(kept.contains(&second.to_string()));
        assert!(!kept.contains(&first.to_string()));
        assert!(
            project_profile_root(&data, id)
                .join(third.to_string())
                .join("templat.sas7bitm")
                .is_file(),
            "an explicit layer selection is honoured"
        );
    }

    #[test]
    fn a_failed_capture_keeps_the_previous_revision_and_its_metadata() {
        use std::os::windows::fs::OpenOptionsExt;

        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("v4");
        fs::create_dir_all(&data).unwrap();
        let id = Uuid::new_v4();
        seeded_project(&data, id);
        let source = source_with_every_catalog(temp.path());
        let first = capture_project_profile(&data, id, &source, None).unwrap();

        let locked = File::options()
            .read(true)
            .share_mode(0)
            .open(source.join("profile.sas7bcat"))
            .unwrap();
        let error = capture_project_profile(&data, id, &source, None).unwrap_err();
        assert_eq!(error.code, "profileLocked");

        let stored: Vec<Project> = read_json(&data.join("projects.json")).unwrap();
        assert_eq!(stored[0].profile.as_ref().unwrap().revision, Some(first));
        assert_eq!(revision_names(&project_profile_root(&data, id)).len(), 1);
        drop(locked);
    }

    #[test]
    fn a_project_without_project_level_preferences_reports_no_layers() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("v4");
        fs::create_dir_all(&data).unwrap();
        assert_eq!(project_layers(&data, Uuid::new_v4()), None);
    }
}
