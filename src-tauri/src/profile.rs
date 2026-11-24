use crate::issue::Issue;
use serde::{Deserialize, Serialize};
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

/// Window geometry, function keys, menus and the per-printer fonts: what a user
/// means by "my layout".
pub const INTERFACE: &[&str] = &["profile.sas7bcat", "profile2.sas7bcat"];
/// Library assignments, Explorer options and print setup.
pub const ENVIRONMENT: &[&str] = &["regstry.sas7bitm"];
/// Compiled ODS templates and styles. This is user content, not a preference.
pub const TEMPLATES: &[&str] = &["templat.sas7bitm"];

/// The four catalog files SAS writes into its user folder, in layer order.
///
/// Deliberately excluded, so nobody "completes" the list by accident:
/// `profbak.sas7bcat` is SAS's own fallback copy, `folder.sas7bcat` and
/// `gng.sas7bcat` hold ASSIST and Graph-N-Go resources, and `parms.sas7bcat`
/// holds ASSIST parameters (written by a different machine on this host).
pub const FILES: &[&str] = &[
    "profile.sas7bcat",
    "profile2.sas7bcat",
    "regstry.sas7bitm",
    "templat.sas7bitm",
];

/// Which layers of a user folder a snapshot should carry.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Layers {
    pub interface: bool,
    pub environment: bool,
    pub templates: bool,
}

impl Layers {
    pub const ALL: Self = Self {
        interface: true,
        environment: true,
        templates: true,
    };

    /// The default carries the interface and the environment layers: window
    /// layout, menus, function keys and the Explorer options a user tunes are
    /// what "my settings" means. Templates stay out: they are user content, not
    /// a preference, and they are the largest of the four.
    pub const DEFAULT: Self = Self {
        interface: true,
        environment: true,
        templates: false,
    };

    pub fn includes(self, name: &str) -> bool {
        (self.interface && INTERFACE.contains(&name))
            || (self.environment && ENVIRONMENT.contains(&name))
            || (self.templates && TEMPLATES.contains(&name))
    }

    pub fn selected(self) -> impl Iterator<Item = &'static str> {
        FILES
            .iter()
            .copied()
            .filter(move |name| self.includes(name))
    }
}
const REPARSE_POINT: u32 = 0x400;
const FILE_SHARE_READ: u32 = 1;

pub fn regular_directory(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_dir() && metadata.file_attributes() & REPARSE_POINT == 0)
}

/// The whitelisted catalogs `directory` holds, in whitelist order.
pub fn catalog_files(directory: &Path) -> Vec<&'static str> {
    FILES
        .iter()
        .copied()
        .filter(|name| directory.join(name).is_file())
        .collect()
}

/// Which layers a set of catalog names covers.
pub fn layers_of(names: &[String]) -> Layers {
    Layers {
        interface: names.iter().any(|name| INTERFACE.contains(&name.as_str())),
        environment: names
            .iter()
            .any(|name| ENVIRONMENT.contains(&name.as_str())),
        templates: names.iter().any(|name| TEMPLATES.contains(&name.as_str())),
    }
}

pub fn has_catalog_files(directory: &Path) -> bool {
    FILES.iter().any(|name| directory.join(name).is_file())
}

/// True when `directory` holds at least one catalog that `layers` would copy.
/// A session that wrote nothing the project asked for must not be captured —
/// an empty capture would pin the folder for a retry that can never succeed.
pub fn carries_preference(directory: &Path, layers: Layers) -> bool {
    layers.selected().any(|name| directory.join(name).is_file())
}

pub fn copy_profile(source: &Path, destination: &Path) -> Result<(), Issue> {
    copy_profile_layers(source, destination, Layers::ALL)
}

pub fn copy_profile_layers(source: &Path, destination: &Path, layers: Layers) -> Result<(), Issue> {
    let mut inputs = Vec::new();
    for name in layers.selected() {
        let path = source.join(name);
        match fs::symlink_metadata(&path) {
            Ok(metadata)
                if metadata.is_file() && metadata.file_attributes() & REPARSE_POINT == 0 => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => continue,
            Ok(_) => return Err(Issue::with_detail("profileNotFile", name)),
            Err(error) => {
                return Err(Issue::with_detail(
                    "profileRead",
                    format!("{name}: {error}"),
                ));
            }
        }
        // Hold all handles until the snapshot is complete. A writer can neither
        // open these files mid-copy nor be active when we take the snapshot.
        let file = OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(path)
            .map_err(|error| Issue::with_detail("profileLocked", format!("{name}: {error}")))?;
        inputs.push((name, file));
    }
    if inputs.is_empty() {
        return Err(Issue::new("profileEmpty"));
    }
    for (name, mut input) in inputs {
        let mut output = File::create_new(destination.join(name))
            .map_err(|error| Issue::with_detail("profileCreate", format!("{name}: {error}")))?;
        io::copy(&mut input, &mut output)
            .and_then(|_| output.sync_all())
            .map_err(|error| Issue::with_detail("profileCopy", format!("{name}: {error}")))?;
    }
    Ok(())
}

pub fn verify_writable(path: &Path) -> Result<(), Issue> {
    tempfile::NamedTempFile::new_in(path)
        .and_then(tempfile::NamedTempFile::close)
        .map_err(|error| Issue::with_detail("profileNotWritable", error))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_selected_layer_counts_as_a_carried_preference() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("user");
        fs::create_dir(&directory).unwrap();
        assert!(!carries_preference(&directory, Layers::DEFAULT));
        fs::write(directory.join("templat.sas7bitm"), b"templates").unwrap();
        assert!(
            !carries_preference(&directory, Layers::DEFAULT),
            "templates are not a preference"
        );
        fs::write(directory.join("regstry.sas7bitm"), b"registry").unwrap();
        assert!(carries_preference(&directory, Layers::DEFAULT));
    }

    #[test]
    fn a_folder_reports_the_catalogs_and_layers_it_holds() {
        let temp = tempfile::tempdir().unwrap();
        let directory = temp.path().join("user");
        fs::create_dir(&directory).unwrap();
        fs::write(directory.join("private.sas"), b"script").unwrap();
        fs::write(directory.join("regstry.sas7bitm"), b"registry").unwrap();
        fs::write(directory.join("profile.sas7bcat"), b"settings").unwrap();

        let files = catalog_files(&directory);
        assert_eq!(files, vec!["profile.sas7bcat", "regstry.sas7bitm"]);
        let names: Vec<String> = files.into_iter().map(str::to_owned).collect();
        assert_eq!(
            layers_of(&names),
            Layers {
                interface: true,
                environment: true,
                templates: false,
            }
        );
    }

    #[test]
    fn catalog_files_are_the_documented_four() {
        assert_eq!(FILES.len(), 4);
        for name in [
            "profile.sas7bcat",
            "profile2.sas7bcat",
            "regstry.sas7bitm",
            "templat.sas7bitm",
        ] {
            assert!(FILES.contains(&name), "{name}");
        }
    }

    #[test]
    fn the_layers_partition_the_whitelist() {
        let mut union: Vec<&str> = INTERFACE
            .iter()
            .chain(ENVIRONMENT)
            .chain(TEMPLATES)
            .copied()
            .collect();
        union.sort_unstable();
        let mut sorted = FILES.to_vec();
        sorted.sort_unstable();
        assert_eq!(union, sorted, "every whitelisted file belongs to one layer");
        assert_eq!(Layers::ALL.selected().count(), FILES.len());
        assert_eq!(
            Layers::DEFAULT.selected().count(),
            INTERFACE.len() + ENVIRONMENT.len(),
            "the default carries the interface and the environment layers"
        );
        assert!(
            Layers::DEFAULT.includes("regstry.sas7bitm"),
            "Explorer options live in the environment layer"
        );
        assert!(!Layers::DEFAULT.includes("templat.sas7bitm"));
        assert!(Layers::ALL.includes("templat.sas7bitm"));
    }

    #[test]
    fn the_default_layers_carry_the_explorer_options_and_leave_templates_behind() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        for name in FILES {
            fs::write(source.join(name), name.as_bytes()).unwrap();
        }
        let destination = temp.path().join("destination");
        fs::create_dir(&destination).unwrap();
        copy_profile_layers(&source, &destination, Layers::DEFAULT).unwrap();

        assert!(destination.join("profile.sas7bcat").is_file());
        assert!(destination.join("profile2.sas7bcat").is_file());
        assert!(destination.join("regstry.sas7bitm").is_file());
        assert!(!destination.join("templat.sas7bitm").exists());
    }

    #[test]
    fn a_layer_without_files_is_an_empty_profile() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("regstry.sas7bitm"), b"registry").unwrap();
        let destination = temp.path().join("destination");
        fs::create_dir(&destination).unwrap();

        let interface_only = Layers {
            interface: true,
            environment: false,
            templates: false,
        };
        let error = copy_profile_layers(&source, &destination, interface_only).unwrap_err();
        assert_eq!(error.code, "profileEmpty");
        assert_eq!(fs::read_dir(&destination).unwrap().count(), 0);
    }

    #[test]
    fn copies_only_supported_files_and_destinations_are_independent() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source");
        fs::create_dir(&source).unwrap();
        fs::write(source.join("profile.sas7bcat"), b"baseline").unwrap();
        fs::write(source.join("templat.sas7bitm"), b"templates").unwrap();
        fs::write(source.join("private.sas"), b"script").unwrap();
        let first = temp.path().join("first");
        let second = temp.path().join("second");
        fs::create_dir(&first).unwrap();
        fs::create_dir(&second).unwrap();
        copy_profile(&source, &first).unwrap();
        copy_profile(&source, &second).unwrap();
        fs::write(first.join("profile.sas7bcat"), b"changed").unwrap();
        assert_eq!(
            fs::read(second.join("profile.sas7bcat")).unwrap(),
            b"baseline"
        );
        assert_eq!(
            fs::read(source.join("profile.sas7bcat")).unwrap(),
            b"baseline"
        );
        assert!(!first.join("private.sas").exists());
        assert!(first.join("templat.sas7bitm").is_file());
    }
}
