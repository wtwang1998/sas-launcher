use crate::issue::Issue;
use crate::{
    discovery::{ConfigKey, LaunchConfig},
    process::{self, ProcessIdentity, Status},
    profile::{copy_profile, regular_directory, verify_writable},
    store::{read_json, write_json},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs, io,
    path::{Path, PathBuf},
    process::Child,
};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
enum Lifecycle {
    Preparing,
    Running { identity: ProcessIdentity },
    Exited { code: Option<u32> },
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Manifest {
    id: Uuid,
    key: ConfigKey,
    name: String,
    title: String,
    template_setup: bool,
    #[serde(default)]
    project_id: Option<Uuid>,
    #[serde(default)]
    tracked_project_identity: bool,
    args: Vec<String>,
    lifecycle: Lifecycle,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionSummary {
    pub id: Uuid,
    pub key: ConfigKey,
    pub name: String,
    pub title: String,
    pub profile: PathBuf,
    pub args: Vec<String>,
    pub template_setup: bool,
    pub project_id: Option<Uuid>,
    pub legacy_identity: bool,
    pub status: &'static str,
    pub exit_code: Option<u32>,
}

pub enum Purpose {
    Project(Option<Uuid>),
    Template,
}

pub struct Sessions {
    root: PathBuf,
    children: HashMap<Uuid, Child>,
    pub notices: Vec<Issue>,
}

fn window_title(name: &str, encoding: &str) -> String {
    // SAS parses AWSTITLE from the native command line. A literal double quote
    // can end the value even when Windows argument quoting escaped it.
    format!(
        "{} · {}",
        name.replace('"', "＂"),
        encoding.to_ascii_uppercase()
    )
}

impl Sessions {
    pub fn new(root: PathBuf) -> Self {
        Self {
            root,
            children: HashMap::new(),
            notices: Vec::new(),
        }
    }

    fn directory(&self, id: Uuid) -> PathBuf {
        self.root.join(id.to_string())
    }

    fn read_manifest(&self, id: Uuid) -> Option<Manifest> {
        let dir = self.directory(id);
        if !regular_directory(&self.root) || !regular_directory(&dir) {
            return None;
        }
        let manifest: Manifest = read_json(&dir.join("session.json")).ok()?;
        (manifest.id == id).then_some(manifest)
    }

    fn notice(&mut self, message: Issue) {
        if self.notices.len() >= 8 {
            self.notices.remove(0);
        }
        self.notices.push(message);
    }

    pub fn launch(
        &mut self,
        config: &LaunchConfig,
        name: String,
        template: Option<&Path>,
        purpose: Purpose,
        cwd: &Path,
    ) -> Result<SessionSummary, Issue> {
        if !regular_directory(&self.root) {
            return Err(Issue::new("sessionDirectory"));
        }
        let encoding = config
            .encoding
            .as_deref()
            .ok_or(Issue::new("encodingUnknown"))?;
        let id = Uuid::new_v4();
        let dir = self.directory(id);
        fs::create_dir(&dir).map_err(|error| Issue::with_detail("sessionCreate", error))?;
        let profile = dir.join("sasuser");
        let title = window_title(&name, encoding);
        let args = process::arguments(config, &profile, &title, encoding);
        let mut manifest = Manifest {
            id,
            key: config.key.clone(),
            name,
            title,
            template_setup: matches!(purpose, Purpose::Template),
            project_id: match purpose {
                Purpose::Project(id) => id,
                Purpose::Template => None,
            },
            tracked_project_identity: true,
            args: args
                .iter()
                .map(|arg| arg.to_string_lossy().into_owned())
                .collect(),
            lifecycle: Lifecycle::Preparing,
        };
        let file = dir.join("session.json");
        let result = (|| {
            fs::create_dir(&profile)
                .map_err(|error| Issue::with_detail("sessionProfileCreate", error))?;
            verify_writable(&profile)?;
            if let Some(template) = template {
                copy_profile(template, &profile)?;
            }
            write_json(&file, &manifest)
                .map_err(|error| Issue::with_detail("sessionRecord", error))?;
            process::start(config, &args, cwd)
        })();
        let child = match result {
            Ok(child) => child,
            Err(error) => {
                let _ = self.remove_run(id);
                return Err(error);
            }
        };
        // If durable bookkeeping fails, keep the live handle. The on-disk
        // preparation record prevents recovery from deleting a possibly live run.
        match ProcessIdentity::from_child(&child) {
            Ok(identity) => {
                manifest.lifecycle = Lifecycle::Running { identity };
                if let Err(error) = write_json(&file, &manifest) {
                    self.notice(Issue::with_detail("sessionRecovery", error));
                }
            }
            Err(error) => self.notice(Issue::with_detail("sessionIdentity", error)),
        }
        self.children.insert(id, child);
        Ok(self.describe(manifest, "running", None))
    }

    fn describe(
        &self,
        manifest: Manifest,
        status: &'static str,
        exit_code: Option<u32>,
    ) -> SessionSummary {
        SessionSummary {
            id: manifest.id,
            key: manifest.key,
            name: manifest.name,
            title: manifest.title,
            profile: self.directory(manifest.id).join("sasuser"),
            args: manifest.args,
            template_setup: manifest.template_setup,
            project_id: manifest.project_id,
            legacy_identity: !manifest.tracked_project_identity,
            status,
            exit_code,
        }
    }

    pub fn sweep(&mut self) -> Vec<SessionSummary> {
        if !regular_directory(&self.root) {
            return Vec::new();
        }
        let Ok(entries) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let mut sessions = Vec::new();
        for entry in entries.flatten() {
            let Ok(id) = Uuid::parse_str(&entry.file_name().to_string_lossy()) else {
                continue;
            };
            let Some(mut manifest) = self.read_manifest(id) else {
                continue;
            };
            let status = if let Some(child) = self.children.get_mut(&id) {
                match child.try_wait() {
                    Ok(Some(exit)) => {
                        self.children.remove(&id);
                        Status::Exited(exit.code().map(|code| code as u32))
                    }
                    Ok(None) => Status::Running,
                    Err(_) => Status::Unknown,
                }
            } else {
                match manifest.lifecycle {
                    Lifecycle::Preparing => Status::Unknown,
                    Lifecycle::Running { identity } => identity.status(),
                    Lifecycle::Exited { code } => Status::Exited(code),
                }
            };
            match status {
                Status::Running => sessions.push(self.describe(manifest, "running", None)),
                Status::Unknown => sessions.push(self.describe(manifest, "unknown", None)),
                Status::Exited(code) => {
                    if !matches!(manifest.lifecycle, Lifecycle::Exited { .. }) {
                        manifest.lifecycle = Lifecycle::Exited { code };
                        if write_json(&self.directory(id).join("session.json"), &manifest).is_err()
                        {
                            sessions.push(self.describe(manifest, "unknown", None));
                            continue;
                        }
                        if let Some(code) = code.filter(|code| *code != 0) {
                            self.notice(Issue::with_detail(
                                "sasExit",
                                format!("{} · exit {code}", manifest.title),
                            ));
                        }
                    }
                    if manifest.template_setup {
                        sessions.push(self.describe(manifest, "ready", code));
                    } else if self.remove_run(id).is_err() {
                        sessions.push(self.describe(manifest, "cleanupPending", code));
                    }
                }
            }
        }
        sessions.sort_by(|left, right| left.title.cmp(&right.title).then(left.id.cmp(&right.id)));
        sessions
    }

    pub fn finish_setup(&mut self, id: Uuid, save: bool) -> Result<(ConfigKey, PathBuf), Issue> {
        self.sweep();
        let manifest = self.read_manifest(id).ok_or(Issue::new("setupMissing"))?;
        if !manifest.template_setup || !matches!(manifest.lifecycle, Lifecycle::Exited { .. }) {
            return Err(Issue::new("setupRunning"));
        }
        if save && matches!(manifest.lifecycle, Lifecycle::Exited { code: Some(code) } if code != 0)
        {
            return Err(Issue::new("setupFailed"));
        }
        let profile = self.directory(id).join("sasuser");
        if save && !regular_directory(&profile) {
            return Err(Issue::new("setupDirectory"));
        }
        Ok((manifest.key, profile))
    }

    pub fn retire_setup(&mut self, id: Uuid) -> Result<(), Issue> {
        let mut manifest = self.read_manifest(id).ok_or(Issue::new("setupMissing"))?;
        manifest.template_setup = false;
        write_json(&self.directory(id).join("session.json"), &manifest)
            .map_err(|error| Issue::with_detail("setupRecord", error))?;
        let _ = self.remove_run(id);
        Ok(())
    }

    fn remove_run(&self, id: Uuid) -> io::Result<()> {
        let directory = self.directory(id);
        if !regular_directory(&self.root) || !regular_directory(&directory) {
            return Err(io::Error::other("Not a managed session directory"));
        }
        // Remove the payload first. Keep the recovery record if any SAS file
        // is still locked, so a later sweep can retry the same directory.
        let profile = directory.join("sasuser");
        if profile.exists() && !regular_directory(&profile) {
            return Err(io::Error::other("Not a regular SAS user directory"));
        }
        match fs::remove_dir_all(profile) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => return Err(error),
            _ => {}
        }
        fs::remove_dir_all(directory)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs::File, os::windows::fs::OpenOptionsExt};

    #[test]
    fn window_title_uppercases_only_the_encoding_suffix() {
        for (source, display) in [
            ("utf-8", "UTF-8"),
            ("euc-cn", "EUC-CN"),
            ("wLaTiN1", "WLATIN1"),
        ] {
            assert_eq!(
                window_title("MixedCase 项目", source),
                format!("MixedCase 项目 · {display}")
            );
        }
    }

    #[test]
    fn title_uses_safe_quotes_without_changing_the_saved_project_name() {
        assert_eq!(window_title("项目 \"A\"", "UTF-8"), "项目 ＂A＂ · UTF-8",);
    }
    #[test]
    fn cleanup_keeps_unknown_run_and_retries_locked_exited_run() {
        let temp = tempfile::tempdir().unwrap();
        let mut sessions = Sessions::new(temp.path().to_path_buf());
        let id = Uuid::new_v4();
        let dir = sessions.directory(id);
        let profile = dir.join("sasuser");
        fs::create_dir_all(&profile).unwrap();
        let file = dir.join("session.json");
        let mut manifest = Manifest {
            id,
            key: ConfigKey {
                exe: "sas.exe".into(),
                cfg: "cfg".into(),
            },
            name: "项目".into(),
            title: "项目 · UTF-8".into(),
            template_setup: false,
            project_id: None,
            tracked_project_identity: false,
            args: vec![],
            lifecycle: Lifecycle::Preparing,
        };
        write_json(&file, &manifest).unwrap();
        assert_eq!(sessions.sweep()[0].status, "unknown");
        assert!(profile.exists());

        manifest.lifecycle = Lifecycle::Exited { code: Some(0) };
        write_json(&file, &manifest).unwrap();
        let catalog = profile.join("regstry.sas7bitm");
        fs::write(&catalog, b"settings").unwrap();
        let lock = File::options()
            .read(true)
            .share_mode(1)
            .open(&catalog)
            .unwrap();
        assert_eq!(sessions.sweep()[0].status, "cleanupPending");
        assert!(file.exists());
        drop(lock);
        assert!(sessions.sweep().is_empty());
        assert!(!dir.exists());
    }
}
