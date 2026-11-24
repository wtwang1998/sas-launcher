use crate::{
    issue::{FailureNote, Issue},
    process::{self, ProcessIdentity, Status as Probe},
    profile::{copy_profile, regular_directory, verify_writable},
    store::{read_json, write_json},
};
use serde::{Deserialize, Serialize};
use std::{
    cmp::Reverse,
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    process::Child,
};
use uuid::Uuid;

const MANIFEST: &str = "manifest.json";
const LOG: &str = "SAS.LOG";
const SASUSER: &str = "sasuser";
const KEEP_EXITED: usize = 20;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Purpose {
    Project { id: Option<Uuid> },
    Setup { config: Uuid },
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    Exe,
    Cfg,
    Profile,
    Spawn,
    Started,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Running,
    Exited,
    /// A settings session that has exited and awaits save/discard.
    Setup,
    #[serde(rename = "cleanupPending")]
    CleanupPending,
    Unknown,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Record {
    pub id: Uuid,
    pub name: String,
    pub title: String,
    pub purpose: Purpose,
    pub stage: Stage,
    pub status: Status,
    pub exit_code: Option<u32>,
    /// Unix milliseconds.
    pub started_at: i64,
    pub argv: Vec<String>,
    pub sasuser: PathBuf,
    pub sasuser_writable: bool,
    /// Catalogs the launch copied into `sasuser`; empty for a fresh environment.
    #[serde(default)]
    pub profile_files: Vec<String>,
    pub thin_cfg: Option<PathBuf>,
    pub factory_cfg: PathBuf,
    pub encoding: Option<String>,
    pub locale: Option<String>,
    /// Set when the launch failed before a process existed (exe/cfg/profile).
    #[serde(default)]
    pub failure: Option<FailureNote>,
}

pub struct LaunchRequest {
    pub purpose: Purpose,
    pub name: String,
    pub cfg: PathBuf,
    pub thin_cfg: Option<PathBuf>,
    pub factory_cfg: PathBuf,
    pub encoding: String,
    pub locale: Option<String>,
    /// Snapshot directory whose catalog files seed the new sasuser folder.
    pub profile: Option<PathBuf>,
    pub exe: PathBuf,
}

/// A launch that never reached `sas.exe` (missing installation or configuration).
/// Recording it keeps the diagnostics page honest: the failing stage is visible
/// instead of the launch silently vanishing.
pub struct FailureRequest {
    pub purpose: Purpose,
    pub name: String,
    pub stage: Stage,
    pub failure: Issue,
    pub factory_cfg: Option<PathBuf>,
    pub thin_cfg: Option<PathBuf>,
    pub encoding: Option<String>,
    pub locale: Option<String>,
}

/// Manifest = the public record plus what is needed to detect liveness across
/// restarts (pid + creation time).
#[derive(Clone, Debug, Deserialize, Serialize)]
struct Manifest {
    #[serde(flatten)]
    record: Record,
    #[serde(default)]
    identity: Option<ProcessIdentity>,
    #[serde(default)]
    exit_recorded: bool,
}

pub struct Sessions {
    root: PathBuf,
    children: HashMap<Uuid, Child>,
    cache: Vec<Record>,
    failures: Vec<(String, String)>,
}

fn window_title(name: &str, encoding: Option<&str>) -> String {
    // SAS parses AWSTITLE from the native command line. A literal double quote
    // can end the value even when Windows argument quoting escaped it.
    let name = name.replace('"', "＂");
    match encoding.map(str::trim).filter(|value| !value.is_empty()) {
        Some(encoding) => format!("{name} · {}", encoding.to_ascii_uppercase()),
        None => name,
    }
}

fn now_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis() as i64)
        .unwrap_or_default()
}

fn is_exited(status: Status) -> bool {
    matches!(
        status,
        Status::Exited | Status::Setup | Status::CleanupPending
    )
}

/// The project a session belongs to, if it is a project launch at all.
fn project_of(record: &Record) -> Option<Uuid> {
    match &record.purpose {
        Purpose::Project { id } => *id,
        Purpose::Setup { .. } => None,
    }
}

impl Sessions {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            children: HashMap::new(),
            cache: Vec::new(),
            failures: Vec::new(),
        }
    }

    pub fn directory(&self, id: Uuid) -> PathBuf {
        self.root.join(id.to_string())
    }

    pub fn log_path(&self, id: Uuid) -> PathBuf {
        self.directory(id).join(LOG)
    }

    fn data_dir(&self) -> PathBuf {
        self.root
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| self.root.clone())
    }

    fn manifest_path(&self, id: Uuid) -> PathBuf {
        self.directory(id).join(MANIFEST)
    }

    fn read_manifest(&self, id: Uuid) -> Option<Manifest> {
        let directory = self.directory(id);
        if !regular_directory(&self.root) || !regular_directory(&directory) {
            return None;
        }
        let manifest: Manifest = read_json(&self.manifest_path(id)).ok()?;
        (manifest.record.id == id).then_some(manifest)
    }

    fn write_manifest(&self, manifest: &Manifest) -> io::Result<()> {
        write_json(&self.manifest_path(manifest.record.id), manifest)
    }

    pub fn launch(&mut self, request: LaunchRequest) -> Result<Record, Issue> {
        if !regular_directory(&self.root) {
            return Err(Issue::new("sessionDirectory"));
        }
        let id = Uuid::new_v4();
        let directory = self.directory(id);
        fs::create_dir(&directory).map_err(|error| Issue::with_detail("sessionCreate", error))?;
        let sasuser = directory.join(SASUSER);
        let title = window_title(&request.name, Some(&request.encoding));
        let arguments = process::arguments(&request.cfg, &sasuser, &title, &directory);
        let mut writable = false;
        let mut record = Record {
            id,
            name: request.name,
            title,
            purpose: request.purpose,
            stage: Stage::Profile,
            status: Status::Running,
            exit_code: None,
            started_at: now_millis(),
            argv: arguments
                .iter()
                .map(|argument| argument.to_string_lossy().into_owned())
                .collect(),
            sasuser: sasuser.clone(),
            sasuser_writable: false,
            profile_files: Vec::new(),
            thin_cfg: request.thin_cfg.clone(),
            factory_cfg: request.factory_cfg.clone(),
            encoding: Some(request.encoding.clone()),
            locale: request.locale.clone(),
            failure: None,
        };
        let prepared = (|| -> Result<(), Issue> {
            fs::create_dir(&sasuser)
                .map_err(|error| Issue::with_detail("sessionProfileCreate", error))?;
            writable = verify_writable(&sasuser).is_ok();
            if let Some(source) = request.profile.as_deref()
                && crate::profile::has_catalog_files(source)
            {
                copy_profile(source, &sasuser)?;
            }
            Ok(())
        })();
        if let Err(error) = prepared {
            record.sasuser_writable = writable;
            record.status = Status::Exited;
            record.stage = Stage::Profile;
            record.failure = Some(FailureNote::from(&error));
            let manifest = Manifest {
                record: record.clone(),
                identity: None,
                exit_recorded: true,
            };
            let _ = self.write_manifest(&manifest);
            return Err(error);
        }
        record.sasuser_writable = writable;
        record.profile_files = crate::profile::catalog_files(&sasuser)
            .into_iter()
            .map(str::to_owned)
            .collect();
        record.stage = Stage::Spawn;
        let manifest = Manifest {
            record: record.clone(),
            identity: None,
            exit_recorded: false,
        };
        if let Err(error) = self.write_manifest(&manifest) {
            let _ = fs::remove_dir_all(&directory);
            return Err(Issue::with_detail("sessionRecord", error));
        }
        let child = match process::start(&request.exe, &arguments, &self.root) {
            Ok(child) => child,
            Err(error) => {
                record.status = Status::Exited;
                record.failure = Some(FailureNote::from(&error));
                let manifest = Manifest {
                    record: record.clone(),
                    identity: None,
                    exit_recorded: true,
                };
                let _ = self.write_manifest(&manifest);
                return Err(error);
            }
        };
        record.stage = Stage::Started;
        record.status = Status::Running;
        let mut manifest = Manifest {
            record: record.clone(),
            identity: None,
            exit_recorded: false,
        };
        match ProcessIdentity::from_child(&child) {
            Ok(identity) => {
                manifest.identity = Some(identity);
                let _ = self.write_manifest(&manifest);
            }
            Err(_) => {
                let _ = self.write_manifest(&manifest);
            }
        }
        self.children.insert(id, child);
        self.cache.insert(0, record.clone());
        Ok(record)
    }

    fn probe(&mut self, manifest: &Manifest) -> Option<Probe> {
        let id = manifest.record.id;
        let mut from_child = None;
        if matches!(manifest.record.status, Status::Running | Status::Unknown)
            && let Some(child) = self.children.get_mut(&id)
        {
            match child.try_wait() {
                Ok(Some(exit)) => {
                    from_child = Some(Probe::Exited(exit.code().map(|code| code as u32)))
                }
                Ok(None) => from_child = Some(Probe::Running),
                Err(_) => from_child = Some(Probe::Unknown),
            }
        }
        if matches!(from_child, Some(Probe::Exited(_))) {
            self.children.remove(&id);
        }
        if from_child.is_some() {
            return from_child;
        }
        if !matches!(manifest.record.status, Status::Running | Status::Unknown) {
            return None;
        }
        manifest.identity.map(ProcessIdentity::status)
    }

    fn refresh(&mut self) {
        let mut records = Vec::new();
        if !regular_directory(&self.root) {
            self.cache.clear();
            return;
        }
        let Ok(entries) = fs::read_dir(&self.root) else {
            self.cache.clear();
            return;
        };
        for entry in entries.flatten() {
            let Ok(id) = Uuid::parse_str(&entry.file_name().to_string_lossy()) else {
                continue;
            };
            let Some(mut manifest) = self.read_manifest(id) else {
                continue;
            };
            let is_setup = matches!(manifest.record.purpose, Purpose::Setup { .. });
            let mut changed = false;
            if let Some(probe) = self.probe(&manifest) {
                match probe {
                    Probe::Running => {
                        if manifest.record.status != Status::Running {
                            manifest.record.status = Status::Running;
                            changed = true;
                        }
                    }
                    Probe::Unknown => {
                        if manifest.record.status != Status::Unknown {
                            manifest.record.status = Status::Unknown;
                            changed = true;
                        }
                    }
                    Probe::Exited(code) => {
                        manifest.record.status = if is_setup {
                            Status::Setup
                        } else {
                            Status::Exited
                        };
                        if code.is_some() {
                            manifest.record.exit_code = code;
                        }
                        changed = true;
                    }
                }
            }
            if is_exited(manifest.record.status) && !manifest.exit_recorded {
                manifest.exit_recorded = true;
                changed = true;
                if manifest.record.exit_code != Some(0) {
                    let encoding = manifest.record.encoding.clone().unwrap_or_default();
                    let locale = manifest.record.locale.clone().unwrap_or_default();
                    if !encoding.is_empty() && !locale.is_empty() {
                        self.failures.push((encoding, locale));
                    }
                    if let Some(code) = manifest.record.exit_code.filter(|code| *code != 0) {
                        manifest.record.failure = Some(FailureNote::from(&Issue::with_detail(
                            "sasExit",
                            format!("exit {code}"),
                        )));
                    }
                }
            }
            if changed {
                let _ = self.write_manifest(&manifest);
            }
            records.push(manifest.record);
        }
        records.sort_by_key(|record| Reverse(record.started_at));
        self.cache = records;
    }

    pub fn records(&mut self) -> Vec<Record> {
        self.refresh();
        self.cache.clone()
    }

    /// Records refreshed by the last `records`/`sweep` call.
    pub fn cached(&self) -> &[Record] {
        &self.cache
    }

    pub fn record(&self, id: Uuid) -> Option<&Record> {
        self.cache.iter().find(|record| record.id == id)
    }

    /// Reap finished sessions. Returns true when a project snapshot was
    /// committed, so the caller knows it must reload its metadata from disk.
    pub fn sweep(&mut self) -> bool {
        if !regular_directory(&self.root) {
            return false;
        }
        self.refresh();
        let cached = self.cache.clone();
        let mut captured = false;
        for record in cached {
            if !is_exited(record.status) {
                continue;
            }
            if matches!(record.purpose, Purpose::Setup { .. }) {
                continue;
            }
            if !regular_directory(&record.sasuser) {
                if record.status == Status::CleanupPending
                    && let Some(mut manifest) = self.read_manifest(record.id)
                {
                    manifest.record.status = Status::Exited;
                    let _ = self.write_manifest(&manifest);
                }
                continue;
            }
            let mut keep_folder = false;
            // Save whatever the session wrote, whatever the exit code: a launch
            // that ends through the window button never reports one, and those
            // catalogs are the only copy of the settings the user tuned.
            if let Some(project) = project_of(&record)
                && let Some(layers) = crate::store::project_layers(&self.data_dir(), project)
                && crate::profile::carries_preference(&record.sasuser, layers)
            {
                match crate::store::capture_project_profile(
                    &self.data_dir(),
                    project,
                    &record.sasuser,
                    Some(layers),
                ) {
                    Ok(_) => captured = true,
                    Err(_) => {
                        // Keep the folder so the next sweep retries: it holds
                        // the only copy of the settings this session produced.
                        if let Some(mut manifest) = self.read_manifest(record.id) {
                            manifest.record.status = Status::CleanupPending;
                            let _ = self.write_manifest(&manifest);
                        }
                        keep_folder = true;
                    }
                }
            }
            if keep_folder {
                continue;
            }
            if fs::remove_dir_all(&record.sasuser).is_err()
                && let Some(mut manifest) = self.read_manifest(record.id)
            {
                manifest.record.status = Status::CleanupPending;
                let _ = self.write_manifest(&manifest);
            }
        }
        self.prune();
        self.refresh();
        captured
    }

    /// Keep the newest `KEEP_EXITED` exited records; running and settings
    /// sessions are never pruned.
    fn prune(&mut self) {
        let mut exited: Vec<(i64, Uuid)> = self
            .cache
            .iter()
            .filter(|record| record.status == Status::Exited)
            .filter(|record| !matches!(record.purpose, Purpose::Setup { .. }))
            .map(|record| (record.started_at, record.id))
            .collect();
        if exited.len() <= KEEP_EXITED {
            return;
        }
        exited.sort_by_key(|entry| Reverse(entry.0));
        for (_, id) in exited.into_iter().skip(KEEP_EXITED) {
            let _ = fs::remove_dir_all(self.directory(id));
            self.cache.retain(|record| record.id != id);
        }
    }

    /// Persist a launch that failed before any process existed.
    pub fn record_failure(&mut self, request: FailureRequest) -> Record {
        let id = Uuid::new_v4();
        let directory = self.directory(id);
        let _ = fs::create_dir_all(&directory);
        let record = Record {
            id,
            name: request.name.clone(),
            title: window_title(&request.name, request.encoding.as_deref()),
            purpose: request.purpose,
            stage: request.stage,
            status: Status::Exited,
            exit_code: None,
            started_at: now_millis(),
            argv: Vec::new(),
            sasuser: directory.join(SASUSER),
            sasuser_writable: false,
            profile_files: Vec::new(),
            thin_cfg: request.thin_cfg,
            factory_cfg: request.factory_cfg.unwrap_or_default(),
            encoding: request.encoding,
            locale: request.locale,
            failure: Some(FailureNote::from(&request.failure)),
        };
        let manifest = Manifest {
            record: record.clone(),
            identity: None,
            exit_recorded: true,
        };
        if self.write_manifest(&manifest).is_ok() {
            self.cache.push(record.clone());
        }
        record
    }

    pub fn setup_session(&self, config: Uuid) -> Option<&Record> {
        self.cache
            .iter()
            .find(|record| matches!(record.purpose, Purpose::Setup { config: id } if id == config))
    }

    pub fn finish_setup(&mut self, config: Uuid, save: bool) -> Result<(), Issue> {
        self.refresh();
        let record = self
            .setup_session(config)
            .cloned()
            .ok_or_else(|| Issue::new("setupMissing"))?;
        if matches!(record.status, Status::Running | Status::Unknown) {
            return Err(Issue::new("setupRunning"));
        }
        if save && record.exit_code != Some(0) {
            return Err(Issue::new("setupFailed"));
        }
        let source = record.sasuser.clone();
        if save && !regular_directory(&source) {
            return Err(Issue::new("setupDirectory"));
        }
        if save {
            let profile_root = self
                .data_dir()
                .join("configurations")
                .join(config.to_string())
                .join("profile");
            fs::create_dir_all(&profile_root)
                .map_err(|error| Issue::with_detail("profileCreate", error))?;
            let revision = Uuid::new_v4();
            let staging = profile_root.join(format!("tmp-{revision}"));
            fs::create_dir(&staging).map_err(|error| Issue::with_detail("profileCreate", error))?;
            if let Err(error) = copy_profile(&source, &staging) {
                let _ = fs::remove_dir_all(&staging);
                return Err(error);
            }
            let target = profile_root.join(revision.to_string());
            if let Err(error) = fs::rename(&staging, &target) {
                let _ = fs::remove_dir_all(&staging);
                return Err(Issue::with_detail("profileCopy", error));
            }
            if let Err(error) =
                crate::store::set_profile_revision_on_disk(&self.data_dir(), config, revision)
            {
                let _ = fs::remove_dir_all(&target);
                return Err(error);
            }
            // Drop the replaced snapshot so repeated saves do not pile up.
            if let Ok(entries) = fs::read_dir(&profile_root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    let stale = path.is_dir()
                        && path != target
                        && entry.file_name().to_string_lossy() != format!("tmp-{revision}");
                    if stale {
                        let _ = fs::remove_dir_all(&path);
                    }
                }
            }
        }
        let _ = fs::remove_dir_all(&source);
        // A finished settings session is not a launch record: drop it so the
        // next setup request opens a fresh session.
        let finished = record.id;
        let _ = fs::remove_dir_all(self.directory(finished));
        self.children.remove(&finished);
        self.cache.retain(|record| record.id != finished);
        self.refresh();
        Ok(())
    }

    pub fn clear(&mut self) -> Result<(), Issue> {
        if !regular_directory(&self.root) {
            self.cache.clear();
            return Ok(());
        }
        self.refresh();
        // Running sessions own live processes and pending settings sessions own
        // the only copy of unsaved catalogs: clearing records must not touch them.
        let keep: Vec<Uuid> = self
            .cache
            .iter()
            .filter(|record| !matches!(record.status, Status::Exited))
            .map(|record| record.id)
            .collect();
        let mut failure = None;
        if let Ok(entries) = fs::read_dir(&self.root) {
            for entry in entries.flatten() {
                let Ok(id) = Uuid::parse_str(&entry.file_name().to_string_lossy()) else {
                    continue;
                };
                if keep.contains(&id) {
                    continue;
                }
                let directory = entry.path();
                if !regular_directory(&directory) {
                    continue;
                }
                if let Err(error) = fs::remove_dir_all(&directory) {
                    failure.get_or_insert_with(|| Issue::with_detail("sessionDirectory", error));
                }
            }
        }
        self.children.retain(|id, _| keep.contains(id));
        self.cache.retain(|record| keep.contains(&record.id));
        match failure {
            Some(issue) => Err(issue),
            None => Ok(()),
        }
    }

    /// Take the launch failures that were observed since the last call.
    pub fn take_failures(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.failures)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::windows::fs::OpenOptionsExt;

    fn test_manifest(
        root: &Path,
        id: Uuid,
        purpose: Purpose,
        status: Status,
        started_at: i64,
    ) -> Manifest {
        let directory = root.join(id.to_string());
        fs::create_dir_all(&directory).unwrap();
        let record = Record {
            id,
            name: format!("session-{started_at}"),
            title: format!("session-{started_at}"),
            purpose,
            stage: Stage::Started,
            status,
            exit_code: Some(0),
            started_at,
            argv: Vec::new(),
            sasuser: directory.join("sasuser"),
            sasuser_writable: true,
            profile_files: Vec::new(),
            thin_cfg: None,
            factory_cfg: PathBuf::from(r"C:\SAS\sasv9.cfg"),
            encoding: Some("utf-8".into()),
            locale: Some("zh_CN".into()),
            failure: None,
        };
        let manifest = Manifest {
            record,
            identity: None,
            exit_recorded: true,
        };
        write_json(&directory.join("manifest.json"), &manifest).unwrap();
        manifest
    }

    #[test]
    fn saving_requires_the_setup_session_to_exit() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("v4/sessions");
        fs::create_dir_all(&root).unwrap();
        let config = Uuid::new_v4();
        let id = Uuid::new_v4();
        let mut manifest = test_manifest(&root, id, Purpose::Setup { config }, Status::Running, 1);
        manifest.record.exit_code = None;
        write_json(&root.join(id.to_string()).join("manifest.json"), &manifest).unwrap();
        let mut sessions = Sessions::new(root);

        assert_eq!(
            sessions.finish_setup(config, true).unwrap_err().code,
            "setupRunning"
        );
        assert_eq!(
            sessions.finish_setup(config, false).unwrap_err().code,
            "setupRunning"
        );
        assert!(sessions.setup_session(config).is_some());

        // Once the process has exited the decision is allowed.
        manifest.record.status = Status::Setup;
        manifest.record.exit_code = Some(3);
        write_json(&sessions.directory(id).join("manifest.json"), &manifest).unwrap();
        assert_eq!(
            sessions.finish_setup(config, true).unwrap_err().code,
            "setupFailed"
        );
    }

    #[test]
    fn failed_snapshot_keeps_the_previous_profile_revision() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("v4");
        let root = data.join("sessions");
        fs::create_dir_all(&root).unwrap();
        let config = Uuid::new_v4();
        let previous = Uuid::new_v4();
        let configuration_dir = data.join("configurations").join(config.to_string());
        let previous_dir = configuration_dir.join("profile").join(previous.to_string());
        fs::create_dir_all(&previous_dir).unwrap();
        fs::write(previous_dir.join("profile.sas7bcat"), b"original").unwrap();
        let configuration = crate::store::UserConfig {
            id: config,
            name: "Mine".into(),
            base: crate::store::ConfigBase::Default,
            encoding: "utf-8".into(),
            locale: "zh_CN".into(),
            revision: Uuid::new_v4(),
            profile_revision: Some(previous),
            factory: PathBuf::from(r"C:\SAS\nls\u8\sasv9.cfg"),
        };
        write_json(&data.join("configurations.json"), &vec![configuration]).unwrap();

        let id = Uuid::new_v4();
        let manifest = test_manifest(&root, id, Purpose::Setup { config }, Status::Setup, 1);
        let sasuser = manifest.record.sasuser.clone();
        fs::create_dir_all(&sasuser).unwrap();
        for name in crate::profile::FILES {
            fs::write(sasuser.join(name), b"settings").unwrap();
        }
        let locked_path = sasuser.join("regstry.sas7bitm");
        let locked = std::fs::File::options()
            .read(true)
            .share_mode(0)
            .open(&locked_path)
            .unwrap();
        let mut sessions = Sessions::new(root);

        let error = sessions.finish_setup(config, true).unwrap_err();
        assert_eq!(error.code, "profileLocked");

        // The old revision and its files are untouched.
        let stored: Vec<crate::store::UserConfig> =
            read_json(&data.join("configurations.json")).unwrap();
        assert_eq!(stored[0].profile_revision, Some(previous));
        assert_eq!(
            fs::read(previous_dir.join("profile.sas7bcat")).unwrap(),
            b"original"
        );
        assert!(sasuser.exists());
        assert_eq!(
            fs::read_dir(configuration_dir.join("profile"))
                .unwrap()
                .flatten()
                .count(),
            1
        );
        drop(locked);
        sessions.finish_setup(config, true).unwrap();
        let stored: Vec<crate::store::UserConfig> =
            read_json(&data.join("configurations.json")).unwrap();
        assert_ne!(stored[0].profile_revision, Some(previous));
        assert!(stored[0].profile_revision.is_some());
    }

    #[test]
    fn clear_keeps_pending_setup_records_that_hold_unsaved_catalogs() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("v4/sessions");
        fs::create_dir_all(&root).unwrap();
        let exited = Uuid::new_v4();
        test_manifest(
            &root,
            exited,
            Purpose::Project { id: None },
            Status::Exited,
            10,
        );
        let setup = Uuid::new_v4();
        test_manifest(
            &root,
            setup,
            Purpose::Setup {
                config: Uuid::new_v4(),
            },
            Status::Setup,
            20,
        );

        let mut sessions = Sessions::new(root.clone());
        sessions.clear().unwrap();

        let remaining: Vec<Uuid> = fs::read_dir(&root)
            .unwrap()
            .flatten()
            .map(|entry| Uuid::parse_str(&entry.file_name().to_string_lossy()).unwrap())
            .collect();
        assert!(!remaining.contains(&exited), "exited records are cleared");
        assert!(
            remaining.contains(&setup),
            "a settings session awaiting save keeps the only copy of its catalogs"
        );
    }

    #[test]
    fn prune_keeps_twenty_newest_exited_and_all_running() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("v4/sessions");
        fs::create_dir_all(&root).unwrap();
        let mut exited = Vec::new();
        for index in 0..25 {
            let id = Uuid::new_v4();
            test_manifest(
                &root,
                id,
                Purpose::Project { id: None },
                Status::Exited,
                index,
            );
            exited.push((index, id));
        }
        let mut running = Vec::new();
        for index in 0..3 {
            let id = Uuid::new_v4();
            test_manifest(
                &root,
                id,
                Purpose::Project { id: None },
                Status::Running,
                1_000 + index,
            );
            running.push(id);
        }
        let setup_id = Uuid::new_v4();
        let setup_config = Uuid::new_v4();
        test_manifest(
            &root,
            setup_id,
            Purpose::Setup {
                config: setup_config,
            },
            Status::Setup,
            2_000,
        );

        let mut sessions = Sessions::new(root.clone());
        sessions.sweep();

        let remaining: Vec<Uuid> = fs::read_dir(&root)
            .unwrap()
            .flatten()
            .map(|entry| Uuid::parse_str(&entry.file_name().to_string_lossy()).unwrap())
            .collect();
        assert_eq!(remaining.len(), KEEP_EXITED + 3 + 1);
        for (index, id) in &exited {
            if *index >= 5 {
                assert!(remaining.contains(id), "kept newest exited {index}");
            } else {
                assert!(!remaining.contains(id), "pruned old exited {index}");
            }
        }
        for id in &running {
            assert!(remaining.contains(id), "running sessions are never pruned");
        }
        assert!(remaining.contains(&setup_id));
    }

    fn seeded_project(data: &Path, id: Uuid, profile: Option<crate::store::ProjectProfile>) {
        write_json(
            &data.join("projects.json"),
            &vec![crate::store::Project {
                id,
                name: "Project".into(),
                note: None,
                config: None,
                profile,
            }],
        )
        .unwrap();
    }

    #[test]
    fn an_exited_project_session_captures_its_profile_before_cleanup() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("v4");
        let root = data.join("sessions");
        fs::create_dir_all(&root).unwrap();
        let project = Uuid::new_v4();
        seeded_project(
            &data,
            project,
            Some(crate::store::ProjectProfile {
                revision: None,
                origin: None,
                layers: crate::profile::Layers::DEFAULT,
                fresh: true,
            }),
        );
        let id = Uuid::new_v4();
        let manifest = test_manifest(
            &root,
            id,
            Purpose::Project { id: Some(project) },
            Status::Exited,
            5,
        );
        let sasuser = manifest.record.sasuser.clone();
        fs::create_dir_all(&sasuser).unwrap();
        fs::write(sasuser.join("profile.sas7bcat"), b"settings").unwrap();

        let mut sessions = Sessions::new(root);
        assert!(sessions.sweep(), "sweep reports the committed snapshot");
        assert!(!sasuser.exists(), "the folder is reaped after the capture");

        let stored: Vec<crate::store::Project> = read_json(&data.join("projects.json")).unwrap();
        let revision = stored[0]
            .profile
            .as_ref()
            .and_then(|profile| profile.revision)
            .unwrap();
        let snapshot = data
            .join("projects")
            .join(project.to_string())
            .join("profile")
            .join(revision.to_string());
        assert_eq!(
            fs::read(snapshot.join("profile.sas7bcat")).unwrap(),
            b"settings"
        );
        assert!(!snapshot.join("regstry.sas7bitm").exists());
    }

    #[test]
    fn an_exited_session_without_an_exit_code_still_captures_its_profile() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("v4");
        let root = data.join("sessions");
        fs::create_dir_all(&root).unwrap();
        let project = Uuid::new_v4();
        seeded_project(
            &data,
            project,
            Some(crate::store::ProjectProfile {
                revision: None,
                origin: None,
                layers: crate::profile::Layers::DEFAULT,
                fresh: true,
            }),
        );
        let id = Uuid::new_v4();
        let mut manifest = test_manifest(
            &root,
            id,
            Purpose::Project { id: Some(project) },
            Status::Exited,
            5,
        );
        // Closing the SAS window leaves the probe without an exit code, yet the
        // catalogs it wrote are still the only copy of what the user tuned.
        manifest.record.exit_code = None;
        write_json(&root.join(id.to_string()).join("manifest.json"), &manifest).unwrap();
        let sasuser = manifest.record.sasuser.clone();
        fs::create_dir_all(&sasuser).unwrap();
        fs::write(sasuser.join("profile.sas7bcat"), b"settings").unwrap();

        let mut sessions = Sessions::new(root);
        assert!(
            sessions.sweep(),
            "an unknown exit code still saves the profile"
        );
        assert!(!sasuser.exists(), "the folder is reaped after the capture");
        let stored: Vec<crate::store::Project> = read_json(&data.join("projects.json")).unwrap();
        let revision = stored[0]
            .profile
            .as_ref()
            .and_then(|profile| profile.revision)
            .expect("the snapshot is committed");
        let snapshot = data
            .join("projects")
            .join(project.to_string())
            .join("profile")
            .join(revision.to_string());
        assert_eq!(
            fs::read(snapshot.join("profile.sas7bcat")).unwrap(),
            b"settings"
        );
    }

    #[test]
    fn a_session_that_saved_no_wanted_layer_is_reaped_without_a_capture() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("v4");
        let root = data.join("sessions");
        fs::create_dir_all(&root).unwrap();
        let project = Uuid::new_v4();
        seeded_project(
            &data,
            project,
            Some(crate::store::ProjectProfile {
                revision: None,
                origin: None,
                layers: crate::profile::Layers::DEFAULT,
                fresh: true,
            }),
        );
        let id = Uuid::new_v4();
        let manifest = test_manifest(
            &root,
            id,
            Purpose::Project { id: Some(project) },
            Status::Exited,
            5,
        );
        let sasuser = manifest.record.sasuser.clone();
        fs::create_dir_all(&sasuser).unwrap();
        fs::write(sasuser.join("templat.sas7bitm"), b"templates").unwrap();

        let mut sessions = Sessions::new(root);
        assert!(
            !sessions.sweep(),
            "nothing of the wanted layers was written"
        );
        assert!(
            !sasuser.exists(),
            "an empty capture does not pin the folder"
        );
        let stored: Vec<crate::store::Project> = read_json(&data.join("projects.json")).unwrap();
        assert!(stored[0].profile.as_ref().unwrap().revision.is_none());
    }

    #[test]
    fn a_project_without_project_level_preferences_is_cleaned_up_untouched() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("v4");
        let root = data.join("sessions");
        fs::create_dir_all(&root).unwrap();
        let project = Uuid::new_v4();
        seeded_project(&data, project, None);
        let id = Uuid::new_v4();
        let manifest = test_manifest(
            &root,
            id,
            Purpose::Project { id: Some(project) },
            Status::Exited,
            5,
        );
        let sasuser = manifest.record.sasuser.clone();
        fs::create_dir_all(&sasuser).unwrap();
        fs::write(sasuser.join("profile.sas7bcat"), b"settings").unwrap();

        let mut sessions = Sessions::new(root);
        assert!(!sessions.sweep());
        assert!(!sasuser.exists());
        let stored: Vec<crate::store::Project> = read_json(&data.join("projects.json")).unwrap();
        assert!(
            stored[0].profile.is_none(),
            "legacy projects are not upgraded"
        );
        assert!(!data.join("projects").join(project.to_string()).exists());
    }

    #[test]
    fn a_failed_capture_keeps_the_folder_for_the_next_sweep() {
        let temp = tempfile::tempdir().unwrap();
        let data = temp.path().join("v4");
        let root = data.join("sessions");
        fs::create_dir_all(&root).unwrap();
        let project = Uuid::new_v4();
        seeded_project(
            &data,
            project,
            Some(crate::store::ProjectProfile {
                revision: None,
                origin: None,
                layers: crate::profile::Layers::DEFAULT,
                fresh: true,
            }),
        );
        let id = Uuid::new_v4();
        let manifest = test_manifest(
            &root,
            id,
            Purpose::Project { id: Some(project) },
            Status::Exited,
            5,
        );
        let sasuser = manifest.record.sasuser.clone();
        fs::create_dir_all(&sasuser).unwrap();
        fs::write(sasuser.join("profile.sas7bcat"), b"settings").unwrap();
        let locked = std::fs::File::options()
            .read(true)
            .share_mode(0)
            .open(sasuser.join("profile.sas7bcat"))
            .unwrap();

        let mut sessions = Sessions::new(root.clone());
        assert!(!sessions.sweep());
        assert!(sasuser.exists(), "the only copy of the settings survives");
        let stored: Vec<crate::store::Project> = read_json(&data.join("projects.json")).unwrap();
        assert!(stored[0].profile.as_ref().unwrap().revision.is_none());
        let manifest: Manifest =
            read_json(&root.join(id.to_string()).join("manifest.json")).unwrap();
        assert_eq!(manifest.record.status, Status::CleanupPending);

        drop(locked);
        assert!(sessions.sweep(), "the retry commits the snapshot");
        assert!(!sasuser.exists());
    }
}
