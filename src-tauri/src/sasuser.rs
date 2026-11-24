//! Discovery of the folder SAS keeps its per-user catalogs in.
//!
//! SAS resolves `-SASUSER "?FOLDERID_Documents\My SAS Files\9.4"`, so the
//! folder lives under the shell's Documents and holds one directory per
//! installed version. Only those version directories are offered: archives and
//! hand-made copies stay reachable through a manual browse.

use crate::profile::{FILES, has_catalog_files};
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};
use windows_sys::Win32::{
    Foundation::S_OK,
    System::Com::CoTaskMemFree,
    UI::Shell::{FOLDERID_Documents, KF_FLAG_DEFAULT, SHGetKnownFolderPath},
};

/// The folder SAS creates inside Documents.
pub const FOLDER: &str = "My SAS Files";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Candidate {
    pub path: PathBuf,
    /// Directory name as SAS writes it (`9.4`, `9.3_1`, ...).
    pub version: String,
    /// How many of the four whitelisted catalogs are present.
    pub files: usize,
    /// Unix milliseconds of the newest catalog file, or 0 when none is dated.
    pub modified: i64,
}

/// `Documents` resolved through the shell, so a redirected folder still works.
pub fn documents_dir() -> Option<PathBuf> {
    // SAFETY: the shell hands back one CoTaskMem allocation, which is freed
    // exactly once here, and the string is only read up to its terminator.
    unsafe {
        let mut raw: *mut u16 = std::ptr::null_mut();
        let status = SHGetKnownFolderPath(
            &FOLDERID_Documents,
            KF_FLAG_DEFAULT as u32,
            std::ptr::null_mut(),
            &mut raw,
        );
        if status != S_OK || raw.is_null() {
            return None;
        }
        let mut length = 0;
        while *raw.add(length) != 0 {
            length += 1;
        }
        let text = String::from_utf16_lossy(std::slice::from_raw_parts(raw, length));
        CoTaskMemFree(raw.cast());
        (!text.is_empty()).then(|| PathBuf::from(text))
    }
}

/// Version folders under the real Documents folder, best candidate first.
pub fn candidates(preferred: Option<&str>) -> Vec<Candidate> {
    documents_dir()
        .map(|documents| candidates_in(&documents.join(FOLDER), preferred))
        .unwrap_or_default()
}

/// Every version folder under `root` that actually holds catalogs. Anything
/// else (`9.4-初始`, `9.4.zip`, a sibling project's test fixtures) is skipped
/// because it is not where SAS writes.
pub fn candidates_in(root: &Path, preferred: Option<&str>) -> Vec<Candidate> {
    let Ok(entries) = fs::read_dir(root) else {
        return Vec::new();
    };
    let preferred = preferred.and_then(parse_version);
    let mut found: Vec<(Vec<u64>, Candidate)> = entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let version = parse_version(&name)?;
            let directory = entry.path();
            if !directory.is_dir() || !has_catalog_files(&directory) {
                return None;
            }
            Some((version, inspect(directory, name)))
        })
        .collect();
    // The version the installation itself uses wins, then the newest version,
    // then the folder touched most recently.
    found.sort_by(|left, right| {
        let left_preferred = preferred.as_ref() == Some(&left.0);
        let right_preferred = preferred.as_ref() == Some(&right.0);
        right_preferred
            .cmp(&left_preferred)
            .then_with(|| right.0.cmp(&left.0))
            .then_with(|| right.1.modified.cmp(&left.1.modified))
    });
    found.into_iter().map(|(_, candidate)| candidate).collect()
}

/// The version directory the configured SAS executable lives in.
pub fn version_from_exe(exe: &Path) -> Option<String> {
    exe.parent()
        .and_then(Path::file_name)
        .map(|name| name.to_string_lossy().into_owned())
        .filter(|name| parse_version(name).is_some())
}

/// `9.4` and `9.3_1` are version folders; `9.4-初始` and `9.4.zip` are not.
fn parse_version(name: &str) -> Option<Vec<u64>> {
    let (main, patch) = match name.split_once('_') {
        Some((main, patch)) => (main, Some(patch)),
        None => (name, None),
    };
    let mut parts = Vec::new();
    for part in main.split('.') {
        parts.push(part.parse::<u64>().ok()?);
    }
    if parts.len() < 2 {
        return None;
    }
    if let Some(patch) = patch {
        parts.push(patch.parse().ok()?);
    }
    Some(parts)
}

fn inspect(directory: PathBuf, version: String) -> Candidate {
    let mut files = 0;
    let mut modified = 0;
    for name in FILES {
        let Ok(metadata) = fs::metadata(directory.join(name)) else {
            continue;
        };
        if !metadata.is_file() {
            continue;
        }
        files += 1;
        if let Ok(time) = metadata.modified()
            && let Ok(since) = time.duration_since(UNIX_EPOCH)
        {
            modified = modified.max(since.as_millis() as i64);
        }
    }
    Candidate {
        path: directory,
        version,
        files,
        modified,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_catalogs(directory: &Path, names: &[&str]) {
        fs::create_dir_all(directory).unwrap();
        for name in names {
            fs::write(directory.join(name), b"settings").unwrap();
        }
    }

    #[test]
    fn only_version_directories_are_parsed() {
        assert_eq!(parse_version("9.4"), Some(vec![9, 4]));
        assert_eq!(parse_version("9.3_1"), Some(vec![9, 3, 1]));
        assert_eq!(parse_version("9.4-初始"), None);
        assert_eq!(parse_version("9.4.zip"), None);
        assert_eq!(parse_version("9"), None);
        assert_eq!(parse_version("Codex"), None);
    }

    #[test]
    fn archives_and_empty_folders_are_not_offered() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("My SAS Files");
        write_catalogs(&root.join("9.4"), &["profile.sas7bcat", "regstry.sas7bitm"]);
        write_catalogs(&root.join("9.4-初始"), &["profile.sas7bcat"]);
        fs::create_dir_all(root.join("9.3")).unwrap();
        fs::write(root.join("9.4.zip"), b"archive").unwrap();

        let candidates = candidates_in(&root, None);
        let versions: Vec<&str> = candidates
            .iter()
            .map(|candidate| candidate.version.as_str())
            .collect();
        assert_eq!(versions, vec!["9.4"]);
        assert_eq!(candidates[0].files, 2);
    }

    #[test]
    fn the_installation_version_wins_over_the_highest_version() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("My SAS Files");
        write_catalogs(&root.join("9.3"), &["profile.sas7bcat"]);
        write_catalogs(&root.join("9.4"), &["profile.sas7bcat"]);

        let versions: Vec<String> = candidates_in(&root, Some("9.3"))
            .into_iter()
            .map(|candidate| candidate.version)
            .collect();
        assert_eq!(versions, vec!["9.3", "9.4"]);

        let versions: Vec<String> = candidates_in(&root, None)
            .into_iter()
            .map(|candidate| candidate.version)
            .collect();
        assert_eq!(versions, vec!["9.4", "9.3"]);
    }

    #[test]
    fn a_missing_root_yields_no_candidates() {
        let temp = tempfile::tempdir().unwrap();
        assert!(candidates_in(&temp.path().join("nowhere"), None).is_empty());
    }

    #[test]
    fn the_installation_version_comes_from_the_executable_directory() {
        let exe = Path::new(r"C:\Program Files\SASHome\SASFoundation\9.4\sas.exe");
        assert_eq!(version_from_exe(exe).as_deref(), Some("9.4"));
        let exe = Path::new(r"C:\Program Files\SASHome\SASFoundation\sas.exe");
        assert_eq!(version_from_exe(exe), None);
    }
}
