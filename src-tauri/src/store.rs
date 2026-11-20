use crate::issue::Issue;
use crate::{
    discovery::{ConfigKey, ConfigRef},
    language::LanguagePreference,
    profile::{copy_profile, regular_directory},
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::{
    fs::{self, File},
    io::{self, Write},
    os::windows::fs::MetadataExt,
    path::{Path, PathBuf},
};
use tempfile::{Builder, NamedTempFile};
use uuid::Uuid;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Project {
    pub id: Uuid,
    pub name: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub config: Option<ConfigRef>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum TemplateSource {
    #[default]
    Existing,
    Factory,
    Imported {
        directory: PathBuf,
    },
    Customized,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct Template {
    key: ConfigKey,
    revision: Option<Uuid>,
    #[serde(default)]
    source: TemplateSource,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ManagedConfig {
    pub id: Uuid,
    pub name: String,
    pub base: ConfigKey,
    #[serde(default)]
    pub encoding: Option<String>,
    #[serde(default)]
    pub locale: Option<String>,
    pub revision: Uuid,
    pub number: u32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(default)]
pub struct Settings {
    pub exe: Option<PathBuf>,
    pub projects: Vec<Project>,
    templates: Vec<Template>,
    pub language: LanguagePreference,
}

pub struct Store {
    pub root: PathBuf,
    pub settings: Settings,
    configurations: Vec<ManagedConfig>,
}

pub fn read_json<T: DeserializeOwned>(path: &Path) -> io::Result<T> {
    let file = File::open(path)?;
    Ok(serde_json::from_reader(io::BufReader::new(file))?)
}

pub fn write_json(path: &Path, value: &impl Serialize) -> io::Result<()> {
    let mut tmp = NamedTempFile::new_in(path.parent().unwrap())?;
    serde_json::to_writer_pretty(&mut tmp, value)?;
    tmp.as_file().sync_all()?;
    tmp.persist(path)?;
    Ok(())
}

impl Store {
    pub fn open(root: PathBuf) -> Result<Self, Issue> {
        fs::create_dir_all(&root).map_err(|error| Issue::with_detail("dataCreate", error))?;
        if !regular_directory(&root) {
            return Err(Issue::new("dataReparse"));
        }
        for folder in ["templates", "sessions", "configurations"] {
            let path = root.join(folder);
            fs::create_dir_all(&path).map_err(|error| Issue::with_detail("dataCreate", error))?;
            if !regular_directory(&path) {
                return Err(Issue::new("dataReparse"));
            }
        }
        let settings = match read_json(&root.join("settings.json")) {
            Ok(settings) => settings,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Settings::default(),
            Err(error) => return Err(Issue::with_detail("settingsRead", error)),
        };
        let configurations = match read_json(&root.join("configurations.json")) {
            Ok(configurations) => configurations,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(Issue::with_detail("settingsRead", error)),
        };
        Ok(Self {
            root,
            settings,
            configurations,
        })
    }

    pub fn update(&mut self, edit: impl FnOnce(&mut Settings)) -> Result<(), Issue> {
        let mut next = self.settings.clone();
        edit(&mut next);
        write_json(&self.root.join("settings.json"), &next)
            .map_err(|error| Issue::with_detail("settingsWrite", error))?;
        self.settings = next;
        Ok(())
    }

    pub fn configuration_key(&self, config: &ManagedConfig) -> ConfigKey {
        ConfigKey {
            exe: config.base.exe.clone(),
            cfg: self.configuration_directory(config.id).join("sasv9.cfg"),
        }
    }

    pub fn configurations(&self) -> &[ManagedConfig] {
        &self.configurations
    }

    fn configuration_directory(&self, id: Uuid) -> PathBuf {
        self.root.join("configurations").join(id.to_string())
    }

    pub fn configuration_file(&self, config: &ManagedConfig) -> PathBuf {
        self.configuration_directory(config.id)
            .join(config.revision.to_string())
            .join("sasv9.cfg")
    }

    pub fn config_directory_available(&self, config: &ManagedConfig) -> bool {
        regular_directory(&self.configuration_directory(config.id))
            && self
                .configuration_file(config)
                .parent()
                .is_some_and(regular_directory)
    }

    pub fn configuration_file_available(&self, config: &ManagedConfig) -> bool {
        self.config_directory_available(config)
            && fs::symlink_metadata(self.configuration_file(config))
                .is_ok_and(|metadata| metadata.is_file() && metadata.file_attributes() & 0x400 == 0)
    }

    pub fn create_config_revision(&self, config: &ManagedConfig) -> Result<(), Issue> {
        let file = self.configuration_file(config);
        let dir = self.configuration_directory(config.id);
        let copy = || -> io::Result<()> {
            if !dir.exists() {
                fs::create_dir(&dir)?;
            }
            if !regular_directory(&dir) {
                return Err(io::Error::other("Invalid managed configuration directory"));
            }
            let mut contents = fs::read(&config.base.cfg)?;
            append_overrides(
                &mut contents,
                config.encoding.as_deref(),
                config.locale.as_deref(),
            );
            let revision = file.parent().unwrap();
            fs::create_dir(revision)?;
            let result = File::create_new(&file).and_then(|mut output| {
                output.write_all(&contents)?;
                output.sync_all()
            });
            if result.is_err() {
                self.remove_config_revision(config);
            }
            result
        };
        copy().map_err(|error| Issue::with_detail("configurationWrite", error))
    }

    pub fn remove_config_revision(&self, config: &ManagedConfig) {
        if self.config_directory_available(config) {
            let file = self.configuration_file(config);
            let _ = fs::remove_file(&file);
            let _ = fs::remove_dir(file.parent().unwrap());
            let _ = fs::remove_dir(self.configuration_directory(config.id));
        }
    }

    pub fn update_configuration(
        &mut self,
        next: ManagedConfig,
        previous: Option<&ManagedConfig>,
        replace_file: bool,
    ) -> Result<(), Issue> {
        if replace_file {
            self.create_config_revision(&next)?;
        }
        let key = self.configuration_key(&next);
        let old_template = replace_file.then(|| self.template_path(&key)).flatten();
        let mut configurations = self.configurations.clone();
        configurations.retain(|item| item.id != next.id);
        configurations.push(next.clone());
        let config_path = self.root.join("configurations.json");
        if let Err(error) = write_json(&config_path, &configurations) {
            if replace_file {
                self.remove_config_revision(&next);
            }
            return Err(Issue::with_detail("settingsWrite", error));
        }
        let result = self.update(|settings| {
            if previous.is_some() && replace_file {
                settings.templates.retain(|item| item.key != key);
            }
        });
        if let Err(error) = result {
            write_json(&config_path, &self.configurations)
                .map_err(|rollback| Issue::with_detail("settingsWrite", rollback))?;
            if replace_file {
                self.remove_config_revision(&next);
            }
            return Err(error);
        }
        self.configurations = configurations;
        if replace_file && let Some(previous) = previous {
            self.remove_config_revision(previous);
        }
        if let Some(path) = old_template {
            let _ = fs::remove_dir_all(path);
        }
        Ok(())
    }

    pub fn delete_configuration(&mut self, config: &ManagedConfig) -> Result<(), Issue> {
        let key = self.configuration_key(config);
        let previous = self.template_path(&key);
        let old_settings = self.settings.clone();
        let mut configurations = self.configurations.clone();
        configurations.retain(|item| item.id != config.id);
        let result = self.update(|settings| settings.templates.retain(|item| item.key != key));
        if let Err(error) = result {
            return Err(error);
        }
        if let Err(error) = write_json(&self.root.join("configurations.json"), &configurations) {
            write_json(&self.root.join("settings.json"), &old_settings)
                .map_err(|rollback| Issue::with_detail("settingsWrite", rollback))?;
            self.settings = old_settings;
            return Err(Issue::with_detail("settingsWrite", error));
        }
        self.configurations = configurations;
        self.remove_config_revision(config);
        if let Some(path) = previous {
            let _ = fs::remove_dir_all(path);
        }
        Ok(())
    }

    pub fn rollback_configuration_creation(&mut self, id: Uuid) -> Result<(), Issue> {
        let config = self
            .configurations
            .iter()
            .find(|config| config.id == id)
            .cloned();
        if let Some(config) = config {
            self.delete_configuration(&config)?;
        }
        Ok(())
    }

    fn template_root(&self) -> PathBuf {
        self.root.join("templates")
    }

    pub fn template_path(&self, key: &ConfigKey) -> Option<PathBuf> {
        let template = self
            .settings
            .templates
            .iter()
            .find(|template| &template.key == key)?;
        let root = self.template_root();
        let revision = root.join(template.revision?.to_string());
        (regular_directory(&root) && regular_directory(&revision)).then_some(revision)
    }

    pub fn template_source(&self, key: &ConfigKey) -> Option<TemplateSource> {
        let template = self
            .settings
            .templates
            .iter()
            .find(|template| &template.key == key)?;
        if matches!(template.source, TemplateSource::Factory) || self.template_path(key).is_some() {
            return Some(template.source.clone());
        }
        None
    }

    pub fn use_factory(&mut self, key: ConfigKey) -> Result<(), Issue> {
        let previous = self.template_path(&key);
        self.update(|settings| {
            settings.templates.retain(|item| item.key != key);
            settings.templates.push(Template {
                key,
                revision: None,
                source: TemplateSource::Factory,
            });
        })?;
        if let Some(old) = previous {
            let _ = fs::remove_dir_all(old);
        }
        Ok(())
    }

    pub fn install_template(
        &mut self,
        key: ConfigKey,
        directory: &Path,
        source: TemplateSource,
    ) -> Result<(), Issue> {
        let root = self.template_root();
        if !regular_directory(&root) {
            return Err(Issue::new("templateDirectory"));
        }
        let staging = Builder::new()
            .prefix("prepare-")
            .tempdir_in(&root)
            .map_err(|error| Issue::with_detail("templateStage", error))?;
        copy_profile(directory, staging.path())?;
        let revision = Uuid::new_v4();
        let target = root.join(revision.to_string());
        let previous = self.template_path(&key);
        fs::rename(staging.path(), &target)
            .map_err(|error| Issue::with_detail("templateWrite", error))?;
        if let Err(error) = self.update(|settings| {
            settings.templates.retain(|template| template.key != key);
            settings.templates.push(Template {
                key,
                revision: Some(revision),
                source,
            });
        }) {
            let _ = fs::remove_dir_all(target);
            return Err(error);
        }
        if let Some(old) = previous {
            let _ = fs::remove_dir_all(old);
        }
        Ok(())
    }
}

fn append_overrides(contents: &mut Vec<u8>, encoding: Option<&str>, locale: Option<&str>) {
    let mut lines = String::new();
    if let Some(encoding) = encoding {
        lines.push_str("-ENCODING ");
        lines.push_str(encoding);
        lines.push('\n');
    }
    if let Some(locale) = locale {
        lines.push_str("-LOCALE ");
        lines.push_str(locale);
        lines.push('\n');
    }
    if lines.is_empty() {
        return;
    }
    let little_endian = contents.starts_with(&[0xff, 0xfe]);
    let big_endian = contents.starts_with(&[0xfe, 0xff]);
    if little_endian || big_endian {
        let newline = if little_endian {
            [0x0a, 0x00]
        } else {
            [0x00, 0x0a]
        };
        let carriage_return = if little_endian {
            [0x0d, 0x00]
        } else {
            [0x00, 0x0d]
        };
        if contents.len() >= 2
            && !contents.ends_with(&newline)
            && !contents.ends_with(&carriage_return)
        {
            contents.extend_from_slice(&newline);
        }
        let words = lines.encode_utf16().flat_map(|word| {
            if little_endian {
                word.to_le_bytes()
            } else {
                word.to_be_bytes()
            }
        });
        contents.extend(words);
    } else {
        if !contents.is_empty() && !contents.ends_with(b"\n") && !contents.ends_with(b"\r") {
            contents.push(b'\n');
        }
        contents.extend_from_slice(lines.as_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn appends_overrides_on_a_new_utf16_line() {
        let mut contents = vec![0xff, 0xfe];
        contents.extend("base".encode_utf16().flat_map(u16::to_le_bytes));

        append_overrides(&mut contents, Some("wlatin1"), Some("en_US"));

        let words: Vec<_> = contents[2..]
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
            .collect();
        assert_eq!(
            String::from_utf16(&words).unwrap(),
            "base\n-ENCODING wlatin1\n-LOCALE en_US\n"
        );
    }

    #[test]
    fn new_project_storage_keeps_optional_note_and_config_reference() {
        let id = Uuid::new_v4();
        let project = Project {
            id,
            name: "Trial".into(),
            note: Some("Safety analysis".into()),
            config: Some(ConfigRef::BuiltIn {
                exe: "sas.exe".into(),
                code: "1d".into(),
            }),
        };
        let json = serde_json::to_value(&project).unwrap();

        assert_eq!(json["note"], "Safety analysis");
        assert_eq!(json["config"]["kind"], "builtIn");
        assert_eq!(json["config"]["code"], "1d");
    }

    #[test]
    fn deleting_a_configuration_removes_it_from_the_separate_store() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("v4");
        let mut store = Store::open(root.clone()).unwrap();
        let base = root.join("base.cfg");
        fs::write(&base, "-ENCODING wlatin1").unwrap();
        let config = ManagedConfig {
            id: Uuid::new_v4(),
            name: "Custom".into(),
            base: ConfigKey {
                exe: "sas.exe".into(),
                cfg: base,
            },
            encoding: Some("wlatin1".into()),
            locale: Some("en_US".into()),
            revision: Uuid::new_v4(),
            number: 1,
        };
        store
            .update_configuration(config.clone(), None, true)
            .unwrap();

        store.delete_configuration(&config).unwrap();

        let stored: Vec<ManagedConfig> = read_json(&root.join("configurations.json")).unwrap();
        assert!(stored.is_empty());
        assert!(store.configurations().is_empty());
    }

    #[test]
    fn configuration_delete_rolls_back_the_config_store_when_settings_commit_fails() {
        use std::os::windows::fs::OpenOptionsExt;

        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("v4");
        let mut store = Store::open(root.clone()).unwrap();
        let base = root.join("base.cfg");
        fs::write(&base, "-ENCODING wlatin1").unwrap();
        let config = ManagedConfig {
            id: Uuid::new_v4(),
            name: "Custom".into(),
            base: ConfigKey {
                exe: "sas.exe".into(),
                cfg: base,
            },
            encoding: Some("wlatin1".into()),
            locale: Some("en_US".into()),
            revision: Uuid::new_v4(),
            number: 1,
        };
        store
            .update_configuration(config.clone(), None, true)
            .unwrap();
        let installed_file = store.configuration_file(&config);
        let settings_lock = File::options()
            .read(true)
            .share_mode(1)
            .open(root.join("settings.json"))
            .unwrap();

        assert!(store.delete_configuration(&config).is_err());
        drop(settings_lock);

        let configurations: Vec<ManagedConfig> =
            read_json(&root.join("configurations.json")).unwrap();
        assert_eq!(configurations.len(), 1);
        assert_eq!(configurations[0].id, config.id);
        assert!(installed_file.is_file());
        assert_eq!(store.configurations().len(), 1);
    }

    #[test]
    fn configuration_metadata_write_failure_preserves_the_previous_file_revision() {
        use std::os::windows::fs::OpenOptionsExt;

        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("v4");
        let mut store = Store::open(root.clone()).unwrap();
        let base = root.join("base.cfg");
        fs::write(&base, "-ENCODING wlatin1").unwrap();
        let mut config = ManagedConfig {
            id: Uuid::new_v4(),
            name: "Original".into(),
            base: ConfigKey {
                exe: "sas.exe".into(),
                cfg: base,
            },
            encoding: Some("wlatin1".into()),
            locale: Some("en_US".into()),
            revision: Uuid::new_v4(),
            number: 1,
        };
        store
            .update_configuration(config.clone(), None, true)
            .unwrap();
        let old_revision = store.configuration_file(&config);
        config.name = "Changed".into();
        config.revision = Uuid::new_v4();
        let lock = File::options()
            .read(true)
            .share_mode(1)
            .open(root.join("configurations.json"))
            .unwrap();

        let previous = store.configurations()[0].clone();
        assert!(
            store
                .update_configuration(config.clone(), Some(&previous), true)
                .is_err()
        );
        drop(lock);

        let saved: Vec<ManagedConfig> = read_json(&root.join("configurations.json")).unwrap();
        assert_eq!(saved[0].name, "Original");
        assert!(old_revision.is_file());
        assert!(!store.configuration_file(&config).exists());
        assert_eq!(store.configurations()[0].name, "Original");
    }

    #[test]
    fn custom_configuration_metadata_is_stored_separately_from_project_settings() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("v4");
        let mut store = Store::open(root.clone()).unwrap();
        let base = root.join("base.cfg");
        fs::write(&base, "-ENCODING wlatin1\n-LOCALE en_US").unwrap();
        let configuration = ManagedConfig {
            id: Uuid::new_v4(),
            name: "Custom".into(),
            base: ConfigKey {
                exe: "sas.exe".into(),
                cfg: base,
            },
            encoding: Some("wlatin1".into()),
            locale: Some("en_US".into()),
            revision: Uuid::new_v4(),
            number: 1,
        };

        store
            .update_configuration(configuration.clone(), None, true)
            .unwrap();

        let settings: serde_json::Value = read_json(&root.join("settings.json")).unwrap();
        let configurations: Vec<ManagedConfig> =
            read_json(&root.join("configurations.json")).unwrap();
        assert!(settings.get("configurations").is_none());
        assert_eq!(configurations.len(), 1);
        assert_eq!(configurations[0].id, configuration.id);
        assert_eq!(configurations[0].encoding.as_deref(), Some("wlatin1"));
        assert_eq!(configurations[0].locale.as_deref(), Some("en_US"));
    }

    #[test]
    fn factory_restore_commits_before_cleaning_previous_template() {
        use std::os::windows::fs::OpenOptionsExt;

        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(temp.path().join("state")).unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("profile.sas7bcat"), b"baseline").unwrap();
        let key = ConfigKey {
            exe: "sas.exe".into(),
            cfg: "sasv9.cfg".into(),
        };
        store
            .install_template(key.clone(), &source, TemplateSource::Customized)
            .unwrap();
        let previous = store.template_path(&key).unwrap();
        let settings = File::options()
            .read(true)
            .share_mode(1)
            .open(store.root.join("settings.json"))
            .unwrap();

        assert!(store.use_factory(key.clone()).is_err());
        assert_eq!(store.template_path(&key), Some(previous.clone()));
        assert!(previous.is_dir());
        drop(settings);

        store.use_factory(key.clone()).unwrap();
        assert!(matches!(
            store.template_source(&key),
            Some(TemplateSource::Factory)
        ));
        assert!(!previous.exists());
        assert!(source.join("profile.sas7bcat").is_file());
    }

    #[test]
    fn existing_template_survives_upgrade_and_defaults_are_scoped() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().to_path_buf();
        Store::open(root.clone()).unwrap();
        let key = ConfigKey {
            exe: "sas.exe".into(),
            cfg: "zh".into(),
        };
        let other = ConfigKey {
            exe: "sas.exe".into(),
            cfg: "u8".into(),
        };
        let revision = Uuid::new_v4();
        fs::create_dir(root.join("templates").join(revision.to_string())).unwrap();
        write_json(
            &root.join("settings.json"),
            &serde_json::json!({
                "templates": [{ "key": key, "revision": revision }],
                "projects": []
            }),
        )
        .unwrap();
        let mut store = Store::open(root).unwrap();
        assert!(matches!(
            store.template_source(&key),
            Some(TemplateSource::Existing)
        ));
        assert!(store.template_source(&other).is_none());
        store.use_factory(other.clone()).unwrap();
        assert!(matches!(
            store.template_source(&other),
            Some(TemplateSource::Factory)
        ));
        assert!(store.template_path(&other).is_none());
        assert!(store.template_path(&key).is_some());
    }
    #[test]
    fn unavailable_snapshot_leaves_previous_revision_installed() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(temp.path().join("state")).unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        let file = source.join("profile.sas7bcat");
        fs::write(&file, b"original").unwrap();
        let key = ConfigKey {
            exe: "sas.exe".into(),
            cfg: "cfg".into(),
        };
        store
            .install_template(key.clone(), &source, TemplateSource::Customized)
            .unwrap();
        let writer = File::options().write(true).open(&file).unwrap();
        assert!(
            store
                .install_template(key.clone(), &source, TemplateSource::Customized)
                .is_err()
        );
        drop(writer);
        let restored = Store::open(store.root.clone()).unwrap();
        let profile = restored
            .template_path(&key)
            .unwrap()
            .join("profile.sas7bcat");
        assert_eq!(fs::read(profile).unwrap(), b"original");
    }

    #[test]
    fn failed_settings_commit_preserves_previous_template() {
        let temp = tempfile::tempdir().unwrap();
        let mut store = Store::open(temp.path().join("state")).unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        let file = source.join("profile.sas7bcat");
        fs::write(&file, b"original").unwrap();
        let key = ConfigKey {
            exe: "sas.exe".into(),
            cfg: "cfg".into(),
        };
        store
            .install_template(key.clone(), &source, TemplateSource::Customized)
            .unwrap();
        let before = store.template_path(&key).unwrap();
        fs::write(&file, b"replacement").unwrap();
        use std::os::windows::fs::OpenOptionsExt;
        let locked = File::options()
            .read(true)
            .share_mode(1)
            .open(store.root.join("settings.json"))
            .unwrap();
        assert!(
            store
                .install_template(key.clone(), &source, TemplateSource::Customized)
                .is_err()
        );
        drop(locked);
        assert_eq!(store.template_path(&key).unwrap(), before);
        assert_eq!(
            fs::read(before.join("profile.sas7bcat")).unwrap(),
            b"original"
        );
    }
}
