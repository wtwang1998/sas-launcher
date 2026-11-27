use crate::{issue::Issue, profile::verify_writable, view::ChainView};
use std::{fs, io::Read, path::Path};

/// The severity carried by the marker that opens a log line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// Whether a session's log could be read at all.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LogState {
    Ok,
    Missing,
    Unreadable,
}

/// One log line that opens with `ERROR:` or `WARNING:`; continuation lines are
/// folded into it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    /// 1-based line number of the marker line.
    pub line: u32,
    pub severity: Severity,
    /// The entry verbatim, marker included, so it can be matched against the file.
    pub text: String,
}

/// What one read pass over a session log found.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogReport {
    pub state: LogState,
    pub errors: u32,
    pub warnings: u32,
    pub entries: Vec<LogEntry>,
    /// The scan stopped early, so the counts are a floor and not a total.
    pub truncated: bool,
    /// Bytes of the file that were actually read, so the panel can say how far
    /// it got without a second pass.
    pub scanned_bytes: u64,
}

/// Above this many entries the counts are a floor, not a total: a log this
/// long is past the point where a list of every row is still a viewer.
pub const MAX_ENTRIES: usize = 2_000;

/// Bytes read before the scan gives up. A session log that is still being
/// written can grow to hundreds of megabytes; the first stretch already holds
/// what the user can act on.
pub const MAX_SCANNED_BYTES: u64 = 8 * 1024 * 1024;

/// Read a SAS log once: the entries and the counts come from the same pass, so
/// they can never disagree. A missing or unreadable log is a state, not an
/// error, and a log too long to read whole says so instead of pretending the
/// counts are complete.
pub fn read_log(path: &Path, encoding: Option<&str>) -> LogReport {
    let mut report = LogReport {
        state: LogState::Ok,
        errors: 0,
        warnings: 0,
        entries: Vec::new(),
        truncated: false,
        scanned_bytes: 0,
    };
    let Ok(file) = fs::File::open(path) else {
        report.state = if path.exists() {
            LogState::Unreadable
        } else {
            LogState::Missing
        };
        return report;
    };
    // One byte more than the cap is asked for, so hitting it is decidable: a
    // file exactly as long as the cap is complete.
    let mut bytes = Vec::new();
    if file
        .take(MAX_SCANNED_BYTES + 1)
        .read_to_end(&mut bytes)
        .is_err()
    {
        report.state = LogState::Unreadable;
        return report;
    }
    if bytes.len() as u64 > MAX_SCANNED_BYTES {
        bytes.truncate(MAX_SCANNED_BYTES as usize);
        report.truncated = true;
    }
    report.scanned_bytes = bytes.len() as u64;

    // A UTF-16 log has two-byte characters, so splitting on a single `0x0A`
    // byte would cut every character in half; decode it whole first.
    let utf16 = bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff]);
    let text = if utf16 {
        decode_log(&bytes, encoding)
    } else {
        String::new()
    };
    let source: &[u8] = if utf16 { text.as_bytes() } else { &bytes };
    let decoded_line = |line: &[u8]| -> String {
        if utf16 {
            String::from_utf8_lossy(line).into_owned()
        } else {
            decode_line(line, encoding)
        }
    };

    let mut folding = false;
    for (index, line) in source.split(|byte| *byte == b'\n').enumerate() {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if let Some(severity) = marker_of(line) {
            if report.entries.len() >= MAX_ENTRIES {
                // Something was cut: the counts no longer cover the log.
                report.truncated = true;
                return report;
            }
            match severity {
                Severity::Error => report.errors += 1,
                Severity::Warning => report.warnings += 1,
            }
            report.entries.push(LogEntry {
                line: index as u32 + 1,
                severity,
                text: decoded_line(line),
            });
            folding = true;
            continue;
        }
        // SAS indents the rest of a message it wrapped; those lines belong to
        // the entry above them, so they are joined back into it. Nothing is
        // inserted at the seam: the pieces already carry the spacing the
        // message had before it was cut.
        let continued = folding
            && line
                .first()
                .is_some_and(|byte| *byte == b' ' || *byte == b'\t');
        if continued {
            if let Some(last) = report.entries.last_mut() {
                let indent = line
                    .iter()
                    .position(|byte| *byte != b' ' && *byte != b'\t')
                    .unwrap_or(line.len());
                last.text.push_str(&decoded_line(&line[indent..]));
            }
            continue;
        }
        folding = false;
    }
    report
}

/// The marker that opens a line, if any. A marker must start the line: SAS
/// indents continuation lines, and a `NOTE:` that merely mentions ERROR is not
/// an error.
fn marker_of(line: &[u8]) -> Option<Severity> {
    for (marker, severity) in [("ERROR:", Severity::Error), ("WARNING:", Severity::Warning)] {
        let head = line.get(..marker.len());
        if head.is_some_and(|head| head.eq_ignore_ascii_case(marker.as_bytes())) {
            return Some(severity);
        }
    }
    None
}

/// Count `ERROR:` / `WARNING:` lines (case-insensitive) in a SAS log.
pub fn scan_log(path: &Path) -> (u32, u32) {
    let report = read_log(path, None);
    (report.errors, report.warnings)
}

/// Decode a log once, line by line, so each line can pick the encoding it was
/// actually written in.
fn decode_log(bytes: &[u8], encoding: Option<&str>) -> String {
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
    let mut text = String::with_capacity(bytes.len());
    for (index, line) in bytes.split(|byte| *byte == b'\n').enumerate() {
        if index > 0 {
            text.push('\n');
        }
        text.push_str(&decode_line(
            line.strip_suffix(b"\r").unwrap_or(line),
            encoding,
        ));
    }
    text
}

fn decode_line(line: &[u8], encoding: Option<&str>) -> String {
    if encoding.is_some_and(is_dbcs) {
        return decode_dbcs(line);
    }
    match std::str::from_utf8(line) {
        Ok(text) => text.to_string(),
        Err(_) => decode_utf8_with_dbcs_runs(line),
    }
}

/// Whether SAS wrote this session's log in a double-byte code page rather than
/// UTF-8. `-LOCALE` does not decide it; the configuration's encoding does.
fn is_dbcs(encoding: &str) -> bool {
    let encoding = encoding.trim().to_ascii_lowercase();
    encoding.starts_with("euc-cn")
        || encoding.starts_with("gb")
        || encoding.starts_with("cp936")
        || encoding.starts_with("windows-936")
}

/// Decode UTF-8 and fall back to the ANSI code page for the runs that are not
/// valid UTF-8. A UTF-8 session still writes the occasional GBK run: SAS echoes
/// a CJK path it was given in argv in the code page, not in the session
/// encoding.
fn decode_utf8_with_dbcs_runs(line: &[u8]) -> String {
    // A runaway write without a newline must not turn this search into a hang.
    const MAX_HYBRID_LINE: usize = 8_192;
    if line.len() > MAX_HYBRID_LINE {
        return String::from_utf8_lossy(line).into_owned();
    }
    let mut text = String::with_capacity(line.len());
    let mut rest = line;
    loop {
        let Err(error) = std::str::from_utf8(rest) else {
            text.push_str(std::str::from_utf8(rest).unwrap_or_default());
            return text;
        };
        let Some((start, end)) = dbcs_run_before(rest, error.valid_up_to()) else {
            text.push_str(&String::from_utf8_lossy(rest));
            return text;
        };
        if let Ok(head) = std::str::from_utf8(&rest[..start]) {
            text.push_str(head);
        }
        text.push_str(&decode_dbcs_strict(&rest[start..end]).unwrap_or_default());
        rest = &rest[end..];
    }
}

/// The ANSI run that covers byte `invalid_at`, as `(start, end)`.
///
/// The run starts at most three bytes back, because the last character before
/// it can be a three-byte UTF-8 sequence — and some GBK pairs are themselves
/// valid UTF-8, so the mismatch alone does not say where the run begins. The
/// longest run that decodes cleanly wins; a real UTF-8 character swallowed by
/// mistake breaks the pair alignment and is rejected.
fn dbcs_run_before(line: &[u8], invalid_at: usize) -> Option<(usize, usize)> {
    for start in invalid_at.saturating_sub(3)..=invalid_at {
        if std::str::from_utf8(&line[..start]).is_err() {
            continue;
        }
        // Take the longest run, not the first one whose tail reads as UTF-8
        // again: 目 is `C4 BF`, a GBK pair that is also the valid UTF-8
        // character Ŀ, so stopping at the first such point splits the run and
        // leaves `Ŀ` in the text. Bytes past the run are either ASCII (which
        // decodes the same either way) or break the GBK walk, so this only
        // grows a run while it stays clean.
        let mut end = line.len();
        while end > invalid_at {
            if resumes_utf8(&line[end..]) && decode_dbcs_strict(&line[start..end]).is_some() {
                return Some((start, end));
            }
            end -= 1;
        }
    }
    None
}

/// Whether a valid UTF-8 character starts at `bytes[0]`.
fn resumes_utf8(bytes: &[u8]) -> bool {
    match std::str::from_utf8(bytes) {
        Ok(_) => true,
        Err(error) => error.valid_up_to() > 0,
    }
}

/// Decode one ANSI double-byte code page. GBK (936) is a superset of GB2312,
/// which is what `euc-cn` means on Windows. `None` when the bytes are not a
/// whole number of valid characters in that code page.
fn decode_dbcs_strict(bytes: &[u8]) -> Option<String> {
    use windows_sys::Win32::Globalization::MultiByteToWideChar;

    const CODE_PAGE_936: u32 = 936;

    let len = i32::try_from(bytes.len()).ok()?;
    if len == 0 {
        return Some(String::new());
    }
    // `MB_ERR_INVALID_CHARS` is documented as not honoured for the ANSI code
    // pages: an odd trailing byte comes back as `?` instead of an error, which
    // would let a half pair claim a run. Check the byte structure first.
    if !is_whole_dbcs(bytes) {
        return None;
    }
    // SAFETY: both buffers are ours, the lengths come from the buffers, and a
    // failed call returns `None` instead of a decoded string.
    let needed = unsafe {
        MultiByteToWideChar(
            CODE_PAGE_936,
            0,
            bytes.as_ptr(),
            len,
            std::ptr::null_mut(),
            0,
        )
    };
    if needed <= 0 {
        return None;
    }
    let mut wide = vec![0u16; needed as usize];
    let written = unsafe {
        MultiByteToWideChar(
            CODE_PAGE_936,
            0,
            bytes.as_ptr(),
            len,
            wide.as_mut_ptr(),
            needed,
        )
    };
    if written <= 0 {
        return None;
    }
    wide.truncate(written as usize);
    Some(String::from_utf16_lossy(&wide))
}

/// Whether the bytes are a whole number of GBK characters. Lead bytes pair
/// with a following byte in `0x40..=0xFE` except `0x7F`; `0x80` is the euro
/// sign and stands alone; `0xFF` never appears.
fn is_whole_dbcs(bytes: &[u8]) -> bool {
    let mut index = 0;
    while index < bytes.len() {
        let byte = bytes[index];
        if byte <= 0x80 {
            index += 1;
        } else if (0x81..=0xfe).contains(&byte) {
            let Some(&trail) = bytes.get(index + 1) else {
                return false;
            };
            if !(0x40..=0xfe).contains(&trail) || trail == 0x7f {
                return false;
            }
            index += 2;
        } else {
            return false;
        }
    }
    true
}

fn decode_dbcs(bytes: &[u8]) -> String {
    decode_dbcs_strict(bytes).unwrap_or_else(|| String::from_utf8_lossy(bytes).into_owned())
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
            profile: None,
        }
    }

    /// The GBK bytes of `统计部`, lifted from the EUC-CN session's log.
    const GBK_TONGJIBU: &[u8] = &[0xcd, 0xb3, 0xbc, 0xc6, 0xb2, 0xbf];
    /// `项目`. The second pair, `C4 BF`, is also the valid UTF-8 character
    /// `Ŀ`, which is what broke the first cut of the run detection.
    const GBK_XIANGMU: &[u8] = &[0xcf, 0xee, 0xc4, 0xbf];

    #[test]
    fn a_whole_log_is_reported_as_scanned_in_full() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("sas.log");
        let bytes = b"ERROR: one\r\nNOTE: two\r\nWARNING: three\r\n";
        fs::write(&log, bytes).unwrap();

        let report = read_log(&log, Some("UTF-8"));
        assert_eq!(report.state, LogState::Ok);
        assert!(!report.truncated);
        assert_eq!(report.scanned_bytes, bytes.len() as u64);
        assert_eq!((report.errors, report.warnings), (1, 1));
    }

    #[test]
    fn the_entry_cap_stops_the_list_and_flags_the_counts_as_a_floor() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("sas.log");
        let mut text = String::new();
        for row in 0..=(MAX_ENTRIES + 5) {
            text.push_str(&format!("ERROR: row {row}\r\n"));
        }
        fs::write(&log, text).unwrap();

        let report = read_log(&log, Some("UTF-8"));
        assert_eq!(report.entries.len(), MAX_ENTRIES);
        assert_eq!(report.errors, MAX_ENTRIES as u32);
        assert!(report.truncated);
    }

    #[test]
    fn a_log_past_the_byte_cap_is_read_up_to_the_cap_only() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("sas.log");
        let padding = "NOTE: still running\r\n";
        let mut text = padding.repeat(MAX_SCANNED_BYTES as usize / padding.len() + 2);
        let padding_len = text.len() as u64;
        text.push_str("ERROR: past the cap\r\n");
        fs::write(&log, text).unwrap();

        let report = read_log(&log, Some("UTF-8"));
        assert_eq!(report.state, LogState::Ok);
        assert!(report.entries.is_empty());
        assert_eq!(report.scanned_bytes, MAX_SCANNED_BYTES);
        assert!(report.truncated);
        assert!(padding_len > MAX_SCANNED_BYTES);
    }

    #[test]
    fn a_dbcs_session_log_is_decoded_with_its_declared_encoding() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("sas.log");
        let mut bytes = b"ERROR: ".to_vec();
        bytes.extend(GBK_TONGJIBU);
        bytes.extend(b"\r\n");
        fs::write(&log, bytes).unwrap();

        let report = read_log(&log, Some("EUC-CN"));
        assert_eq!(report.entries[0].text, "ERROR: 统计部");
    }

    #[test]
    fn a_dbcs_run_inside_a_utf8_line_is_decoded_too() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("sas.log");
        // A UTF-8 session still writes the occasional GBK run: SAS echoes a
        // CJK path it took from argv in the ANSI code page, not the session
        // encoding.
        let mut bytes = "ERROR: 路径 ".as_bytes().to_vec();
        bytes.extend(GBK_TONGJIBU);
        bytes.extend("。".as_bytes());
        bytes.extend(b"\r\n");
        fs::write(&log, bytes).unwrap();

        let report = read_log(&log, Some("UTF-8"));
        assert_eq!(report.entries[0].text, "ERROR: 路径 统计部。");
    }

    #[test]
    fn a_gbk_pair_that_is_also_utf8_does_not_split_the_run() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("sas.log");
        // Real shape, sas.log L27: a UTF-8 session still echoes a path whose
        // CJK part SAS wrote in the ANSI code page.
        let mut bytes = "ERROR: 路径 ".as_bytes().to_vec();
        bytes.extend(GBK_TONGJIBU);
        bytes.extend(b"\\");
        bytes.extend(GBK_XIANGMU);
        bytes.extend("。".as_bytes());
        bytes.extend(b"\r\n");
        fs::write(&log, bytes).unwrap();

        let report = read_log(&log, Some("UTF-8"));
        assert_eq!(report.entries[0].text, "ERROR: 路径 统计部\\项目。");
    }

    #[test]
    fn a_missing_log_is_a_state_not_an_error() {
        let temp = tempfile::tempdir().unwrap();
        let report = read_log(&temp.path().join("missing"), Some("utf-8"));
        assert_eq!(report.state, LogState::Missing);
        assert!(report.entries.is_empty());
        assert_eq!((report.errors, report.warnings), (0, 0));
        assert_eq!(report.scanned_bytes, 0);
        assert!(!report.truncated);
    }

    #[test]
    fn an_unreadable_log_is_its_own_state() {
        let temp = tempfile::tempdir().unwrap();
        // A directory where the log should be cannot be read as bytes.
        let report = read_log(temp.path(), Some("utf-8"));
        assert_eq!(report.state, LogState::Unreadable);
        assert!(report.entries.is_empty());
    }

    #[test]
    fn an_indented_line_continues_the_entry_above_it() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("sas.log");
        // The seams are real: SAS cuts a long message mid-token and pads the
        // pieces it meant to separate with a trailing space.
        let lines = [
            "NOTE: ok",
            "ERROR: 文件正在使用中，E:\\统计部\\项目\\MD\\2024\\05 ",
            "       磐升瑞祥-头皮填充剂\\04 统计分析\\06 TFL\\01 ",
            "       table\\表 6.3.4 不良事件汇总 安全性集.rtf。",
            "NOTE: next",
            "       (not an entry)",
            "WARNING: 没有 Body（主体）文件。",
            "\t不会创建 RTF 输出。",
        ];
        fs::write(&log, lines.join("\r\n")).unwrap();

        let report = read_log(&log, Some("utf-8"));
        assert_eq!(
            report
                .entries
                .iter()
                .map(|entry| (entry.line, entry.severity, entry.text.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (
                    2,
                    Severity::Error,
                    "ERROR: 文件正在使用中，E:\\统计部\\项目\\MD\\2024\\05 磐升瑞祥-头皮填充剂\\04 统计分析\\06 TFL\\01 table\\表 6.3.4 不良事件汇总 安全性集.rtf。"
                ),
                (
                    7,
                    Severity::Warning,
                    "WARNING: 没有 Body（主体）文件。不会创建 RTF 输出。"
                ),
            ]
        );
        assert_eq!((report.errors, report.warnings), (1, 1));
    }

    #[test]
    fn entries_come_only_from_lines_that_start_with_a_marker() {
        let temp = tempfile::tempdir().unwrap();
        let log = temp.path().join("sas.log");
        fs::write(
            &log,
            "NOTE: ok\r\nERROR: bad\r\nWARNING: careful\r\nnote: ERROR: inline\r\n  ERROR: indented\r\nerror: again\r\n",
        )
        .unwrap();
        let report = read_log(&log, Some("utf-8"));
        assert_eq!(report.state, LogState::Ok);
        assert_eq!(
            report
                .entries
                .iter()
                .map(|entry| (entry.line, entry.severity, entry.text.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (2, Severity::Error, "ERROR: bad"),
                (3, Severity::Warning, "WARNING: careful"),
                (6, Severity::Error, "error: again"),
            ]
        );
        assert_eq!((report.errors, report.warnings), (2, 1));
    }

    #[test]
    fn a_recycled_sasuser_is_not_reported_as_read_only() {
        let temp = tempfile::tempdir().unwrap();
        let factory = temp.path().join("sasv9.cfg");
        fs::write(&factory, b"-ENCODING utf-8").unwrap();

        // A finished session recycles its SASUSER folder; that is healthy.
        let issues = self_check(&chain(factory, temp.path().join("recycled")));
        assert!(
            issues.is_empty(),
            "{:?}",
            issues.iter().map(|i| i.code).collect::<Vec<_>>()
        );
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
