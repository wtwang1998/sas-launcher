use crate::{issue::Issue, profile::verify_writable, view::ChainView};
use std::{fs, path::Path};

/// Count `ERROR:` / `WARNING:` lines (case-insensitive) in a SAS log. A
/// missing or unreadable log is not an error: the session simply has no
/// summary yet.
pub fn scan_log(path: &Path) -> (u32, u32) {
    let Ok(bytes) = fs::read(path) else {
        return (0, 0);
    };
    let text = decode_log(&bytes);
    let mut errors = 0;
    let mut warnings = 0;
    for line in text.lines() {
        let line = line.to_ascii_uppercase();
        if line.contains("ERROR:") {
            errors += 1;
        }
        if line.contains("WARNING:") {
            warnings += 1;
        }
    }
    (errors, warnings)
}

fn decode_log(bytes: &[u8]) -> String {
    if let Some(data) = bytes.strip_prefix(&[0xff, 0xfe]) {
        let (words, _) = data.as_chunks::<2>();
        let words: Vec<u16> = words.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
        return String::from_utf16_lossy(&words);
    }
    if let Some(data) = bytes.strip_prefix(&[0xfe, 0xff]) {
        let (words, _) = data.as_chunks::<2>();
        let words: Vec<u16> = words.iter().map(|pair| u16::from_be_bytes(*pair)).collect();
        return String::from_utf16_lossy(&words);
    }
    String::from_utf8_lossy(bytes).into_owned()
}

/// Read-only preflight hints for one session chain.
pub fn self_check(chain: &ChainView) -> Vec<Issue> {
    let mut issues = Vec::new();
    if let Some(factory) = &chain.factory_cfg
        && !is_regular_file(factory)
    {
        issues.push(Issue::with_detail(
            "configUnavailable",
            factory.display().to_string(),
        ));
    }
    // A missing SASUSER means a finished session whose profile was recycled on
    // exit; only an existing but unwritable directory is a real problem.
    if chain.sasuser.exists() && check_writable(&chain.sasuser).is_err() {
        issues.push(Issue::with_detail(
            "profileNotWritable",
            chain.sasuser.display().to_string(),
        ));
    }
    issues
}

fn is_regular_file(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok_and(|metadata| metadata.is_file())
}

fn check_writable(path: &Path) -> Result<(), Issue> {
    verify_writable(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn chain(factory_cfg: PathBuf, sasuser: PathBuf) -> ChainView {
        ChainView {
            label: "chain".into(),
            thin_cfg: None,
            factory_cfg: Some(factory_cfg),
            encoding: Some("utf-8".into()),
            locale: Some("zh_CN".into()),
            sasuser,
            log: None,
        }
    }

    #[test]
    fn missing_log_yields_empty_summary_without_error() {
        let temp = tempfile::tempdir().unwrap();
        assert_eq!(scan_log(&temp.path().join("SAS.LOG")), (0, 0));
        assert_eq!(scan_log(&temp.path().join("missing")), (0, 0));
    }

    #[test]
    fn counts_error_and_warning_lines_case_insensitively() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("SAS.LOG");
        fs::write(
            &log,
            "NOTE: ok\r\nerror: bad\r\nWARNING: careful\r\nError: again\r\nNOTE: ERROR: inline\r\n",
        )
        .unwrap();
        assert_eq!(scan_log(&log), (3, 1));
    }

    #[test]
    fn a_recycled_sasuser_is_not_reported_as_read_only() {
        let temp = tempfile::tempdir().unwrap();
        let factory = temp.path().join("sasv9.cfg");
        fs::write(&factory, b"-ENCODING utf-8").unwrap();

        // A finished session recycles its SASUSER folder; that is healthy.
        let issues = self_check(&chain(factory, temp.path().join("recycled")));
        assert!(issues.is_empty(), "{:?}", issues.iter().map(|i| i.code).collect::<Vec<_>>());
    }

    #[test]
    fn self_check_flags_a_missing_factory_config_and_read_only_sasuser() {
        let temp = tempfile::tempdir().unwrap();
        let missing = temp.path().join("missing").join("sasv9.cfg");
        // A regular file where the folder should be: creating a probe file
        // inside it fails, which is what an unwritable SASUSER looks like.
        let read_only = temp.path().join("sasuser");
        fs::write(&read_only, b"not a directory").unwrap();

        let issues = self_check(&chain(missing, read_only));
        let codes: Vec<&str> = issues.iter().map(|issue| issue.code).collect();
        assert!(codes.contains(&"configUnavailable"), "{codes:?}");
        assert!(codes.contains(&"profileNotWritable"), "{codes:?}");

        let healthy_factory = temp.path().join("sasv9.cfg");
        fs::write(&healthy_factory, b"-ENCODING utf-8").unwrap();
        let healthy_sasuser = temp.path().join("sasuser-ok");
        fs::create_dir(&healthy_sasuser).unwrap();
        assert!(
            self_check(&chain(healthy_factory, healthy_sasuser)).is_empty(),
            "a healthy chain has no preflight hints"
        );
    }
}
