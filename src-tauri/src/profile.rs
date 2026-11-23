use crate::issue::Issue;
use std::{
    fs::{self, File, OpenOptions},
    io,
    os::windows::fs::{MetadataExt, OpenOptionsExt},
    path::Path,
};

/// The four catalog files SAS writes into its user folder.
pub const FILES: &[&str] = &[
    "profile.sas7bcat",
    "profile2.sas7bcat",
    "regstry.sas7bitm",
    "templat.sas7bitm",
];
const REPARSE_POINT: u32 = 0x400;
const FILE_SHARE_READ: u32 = 1;

pub fn regular_directory(path: &Path) -> bool {
    fs::symlink_metadata(path)
        .is_ok_and(|metadata| metadata.is_dir() && metadata.file_attributes() & REPARSE_POINT == 0)
}

pub fn has_catalog_files(directory: &Path) -> bool {
    FILES.iter().any(|name| directory.join(name).is_file())
}

pub fn copy_profile(source: &Path, destination: &Path) -> Result<(), Issue> {
    let mut inputs = Vec::new();
    for name in FILES {
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
