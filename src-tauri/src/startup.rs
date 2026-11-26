//! Migrating a hand-written startup script that still locates itself through
//! `%sysget(SAS_EXECFILEPATH)`.
//!
//! That variable is empty when a script runs as an autoexec file: SAS resets it
//! while a windowed session starts. The launcher therefore hands the path over
//! as `&SASLAUNCHER_STARTUP_FILE`. Rewriting a user's own script is only done in
//! the shapes a person writes by hand; anything else is refused rather than
//! guessed at.

/// The macro variable a migrated script reads.
pub const PATH_VAR: &str = "SASLAUNCHER_STARTUP_FILE";
const OLD: &str = "SAS_EXECFILEPATH";
const OLD_CALL: &[u8] = b"%sysget(SAS_EXECFILEPATH);";
const BOM: [u8; 3] = [0xEF, 0xBB, 0xBF];

#[derive(Debug, PartialEq, Eq)]
pub enum MigrateError {
    /// The script does not read the environment variable at all.
    NothingToDo,
    /// It reads the variable somewhere this migration does not rewrite.
    Unsupported,
}

fn contains_ignore_ascii_case(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window.eq_ignore_ascii_case(needle))
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let start = bytes
        .iter()
        .position(|byte| !byte.is_ascii_whitespace())
        .unwrap_or(bytes.len());
    let end = bytes
        .iter()
        .rposition(|byte| !byte.is_ascii_whitespace())
        .map_or(start, |index| index + 1);
    &bytes[start..end]
}

/// The same check on bytes, for scripts that may not be UTF-8.
pub fn needs_migration_bytes(contents: &[u8]) -> bool {
    contains_ignore_ascii_case(contents, OLD.as_bytes())
        && !contains_ignore_ascii_case(contents, PATH_VAR.as_bytes())
}

/// `%let <name> = %sysget(SAS_EXECFILEPATH);` in any spacing or case, on bytes:
/// the file may be GBK, so nothing is decoded.
fn assignment_name(line: &[u8]) -> Option<&[u8]> {
    let trimmed = trim_ascii(line);
    if trimmed.len() < 4 || !trimmed[..4].eq_ignore_ascii_case(b"%let") {
        return None;
    }
    let rest = &trimmed[4..];
    let equal = rest.iter().position(|byte| *byte == b'=')?;
    let name = trim_ascii(&rest[..equal]);
    if name.is_empty()
        || !name
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        return None;
    }
    trim_ascii(&rest[equal + 1..])
        .eq_ignore_ascii_case(OLD_CALL)
        .then_some(name)
}

fn push(out: &mut Vec<u8>, parts: &[&[u8]], eol: &[u8]) {
    for part in parts {
        out.extend_from_slice(part);
    }
    out.extend_from_slice(eol);
}

fn block(name: &[u8], indent: &[u8], eol: &[u8]) -> Vec<u8> {
    let mut inner = indent.to_vec();
    inner.extend_from_slice(b"    ");
    let mut out = Vec::new();
    push(
        &mut out,
        &[
            indent,
            b"%if %symexist(",
            PATH_VAR.as_bytes(),
            b") %then %do;",
        ],
        eol,
    );
    push(
        &mut out,
        &[&inner, b"%let ", name, b" = &", PATH_VAR.as_bytes(), b";"],
        eol,
    );
    push(&mut out, &[indent, b"%end;"], eol);
    push(&mut out, &[indent, b"%else %do;"], eol);
    push(
        &mut out,
        &[
            &inner,
            b"%let ",
            name,
            b" = %sysget(",
            OLD.as_bytes(),
            b");",
        ],
        eol,
    );
    push(&mut out, &[indent, b"%end;"], b"");
    out
}

/// Rewrites the known self-location assignments, or explains why it will not.
///
/// Works on bytes on purpose: the script may be GBK, and decoding it as UTF-8
/// would replace every Chinese folder name with U+FFFD before writing it back.
/// Everything inserted here is ASCII, so the rest of the file is passed through
/// exactly as it was.
pub fn migrate_bytes(contents: &[u8]) -> Result<Vec<u8>, MigrateError> {
    if !needs_migration_bytes(contents) {
        return Err(MigrateError::NothingToDo);
    }
    // A byte-order mark is not whitespace, so it would hide the first `%let`
    // from the scan. Split it off and put it back untouched.
    let (bom, body) = match contents.strip_prefix(&BOM) {
        Some(rest) => (&BOM[..], rest),
        None => (&[][..], contents),
    };
    let eol: &[u8] = if body.windows(2).any(|window| window == b"\r\n") {
        b"\r\n"
    } else {
        b"\n"
    };
    let mut out = Vec::with_capacity(body.len() + 256);
    let mut replaced = 0;
    for line in body.split_inclusive(|byte| *byte == b'\n') {
        let (content, ending) = match line.strip_suffix(b"\n") {
            Some(rest) => match rest.strip_suffix(b"\r") {
                Some(rest) => (rest, b"\r\n".as_slice()),
                None => (rest, b"\n".as_slice()),
            },
            None => (line, b"".as_slice()),
        };
        match assignment_name(content) {
            Some(name) => {
                let indent_len = content
                    .iter()
                    .take_while(|byte| byte.is_ascii_whitespace())
                    .count();
                out.extend_from_slice(&block(name, &content[..indent_len], eol));
                out.extend_from_slice(ending);
                replaced += 1;
            }
            None => {
                if contains_ignore_ascii_case(content, OLD.as_bytes()) {
                    // Refuse instead of half-fixing the script.
                    return Err(MigrateError::Unsupported);
                }
                out.extend_from_slice(line);
            }
        }
    }
    if replaced == 0 {
        return Err(MigrateError::NothingToDo);
    }
    let mut result = Vec::with_capacity(bom.len() + out.len());
    result.extend_from_slice(bom);
    result.extend_from_slice(&out);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    // The production entry points work on bytes; these two keep the tests
    // written against text.
    fn needs_migration(text: &str) -> bool {
        needs_migration_bytes(text.as_bytes())
    }

    fn migrate(text: &str) -> Result<String, MigrateError> {
        migrate_bytes(text.as_bytes()).map(|bytes| String::from_utf8(bytes).unwrap())
    }

    #[test]
    fn migrates_every_shape_a_hand_written_assignment_takes() {
        let shapes = [
            "%let path = %sysget(SAS_EXECFILEPATH);",
            "%let path=%sysget(SAS_EXECFILEPATH);",
            "%let DIR_CURRENT = %sysget(SAS_EXECFILEPATH);",
            "  %let PATH_CURRENT = %sysget(sas_execfilepath);",
        ];
        for shape in shapes {
            let name = shape
                .split_once("%let")
                .and_then(|(_, rest)| rest.split_once('='))
                .map(|(name, _)| name.trim())
                .unwrap();
            let migrated = migrate(shape).expect("a known shape migrates");
            assert!(
                migrated.contains(&format!("%if %symexist({PATH_VAR}) %then %do;")),
                "{migrated}"
            );
            assert!(
                migrated.contains(&format!("%let {name} = &{PATH_VAR};")),
                "{migrated}"
            );
            assert!(
                migrated.contains(&format!("%let {name} = %sysget({OLD});")),
                "{migrated}"
            );
            // Running the migration twice has nothing left to do.
            assert_eq!(migrate(&migrated), Err(MigrateError::NothingToDo));
        }
    }

    #[test]
    fn refuses_a_script_that_reads_the_variable_somewhere_else() {
        let source = "%let path = %sysget(SAS_EXECFILEPATH);\n%let dir = %substr(%sysget(SAS_EXECFILEPATH), 1, 3);\n";
        assert_eq!(migrate(source), Err(MigrateError::Unsupported));
    }

    #[test]
    fn leaves_the_rest_of_the_script_untouched_and_keeps_crlf() {
        let source = "/*STEP1*/\r\n%let note = keep;\r\n%let path = %sysget(SAS_EXECFILEPATH);\r\n%put &note;\r\n";
        let migrated = migrate(source).expect("migrates");
        assert!(
            migrated.starts_with("/*STEP1*/\r\n%let note = keep;\r\n"),
            "{migrated}"
        );
        assert!(migrated.ends_with("%put &note;\r\n"), "{migrated}");
        assert!(!migrated.contains("\n\n"), "{migrated}");
        // Every inserted line uses the same ending as the file.
        assert_eq!(
            migrated.matches("\r\n").count(),
            source.matches("\r\n").count() + 5
        );
    }

    #[test]
    fn migrates_a_file_whose_comments_are_chinese() {
        // A comment can put the fourth byte inside a multi-byte character; the
        // scan must step back instead of slicing it.
        let source = "/*设置常用文件夹路径*/\r\n%let DIR_CURRENT = %sysget(SAS_EXECFILEPATH);\r\n";
        let migrated = migrate(source).expect("migrates");
        assert!(migrated.starts_with("/*设置常用文件夹路径*/"), "{migrated}");
        assert!(
            migrated.contains("%let DIR_CURRENT = &SASLAUNCHER_STARTUP_FILE;"),
            "{migrated}"
        );
    }

    #[test]
    fn scans_lines_that_start_inside_a_multi_byte_character() {
        // These prefixes are one, two and three bytes wide, so byte index four
        // lands inside the following character.
        for prefix in ["/*设", "/*设置", "/* 设"] {
            assert_eq!(migrate(prefix), Err(MigrateError::NothingToDo), "{prefix}");
        }
    }

    #[test]
    fn keeps_a_byte_order_mark_and_mixed_line_endings() {
        let source = "\u{feff}%let DIR = %sysget(SAS_EXECFILEPATH);\r\n\r\n%put done;\n";
        let migrated = migrate(source).expect("migrates");
        assert!(migrated.starts_with("\u{feff}%if "), "{migrated}");
        assert!(
            migrated.contains("%let DIR = &SASLAUNCHER_STARTUP_FILE;"),
            "{migrated}"
        );
        // Untouched lines keep their own endings, blank line included.
        assert!(migrated.ends_with("\r\n\r\n%put done;\n"), "{migrated}");
    }

    #[test]
    fn migrates_a_gbk_script_without_touching_its_chinese() {
        // "统计分析" as an older project stores it: GBK, not UTF-8.
        let chinese: [u8; 8] = [0xCD, 0xB3, 0xBC, 0xC6, 0xB7, 0xD6, 0xCE, 0xF6];
        let mut source = b"%let DIR_STAT = C:\\04 ".to_vec();
        source.extend_from_slice(&chinese);
        source.extend_from_slice(b";\r\n%let DIR = %sysget(SAS_EXECFILEPATH);\r\n");

        let migrated = migrate_bytes(&source).expect("a gbk script migrates");

        assert!(
            migrated
                .windows(chinese.len())
                .any(|window| window == chinese),
            "the gbk bytes are passed through untouched"
        );
        assert!(
            migrated
                .windows(40)
                .any(|window| window.starts_with(b"%if %symexist(SASLAUNCHER_STARTUP_FI")),
            "the guard block is inserted"
        );
        // Still decodable as GBK, i.e. the file kept its encoding.
        assert!(migrated.windows(8).any(|window| window == chinese));
    }

    #[test]
    fn refuses_a_gbk_script_that_reads_the_variable_elsewhere() {
        let source = b"%let d = %substr(%sysget(SAS_EXECFILEPATH), 1, 3);\r\n";
        assert_eq!(migrate_bytes(source), Err(MigrateError::Unsupported));
    }

    #[test]
    fn the_text_entry_point_agrees_with_the_byte_one() {
        let source = "%let A = %sysget(SAS_EXECFILEPATH);\r\n";
        assert_eq!(
            migrate(source).unwrap().into_bytes(),
            migrate_bytes(source.as_bytes()).unwrap()
        );
    }

    #[test]
    fn a_script_that_never_reads_the_variable_needs_no_migration() {
        assert!(!needs_migration("%let dir = C:/data;\n"));
        assert_eq!(
            migrate("%let dir = C:/data;\n"),
            Err(MigrateError::NothingToDo)
        );
        // Already migrated files carry both names and are left alone.
        let migrated = migrate("%let p = %sysget(SAS_EXECFILEPATH);\n").unwrap();
        assert!(!needs_migration(&migrated));
    }
}
