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

#[derive(Debug, PartialEq, Eq)]
pub enum MigrateError {
    /// The script does not read the environment variable at all.
    NothingToDo,
    /// It reads the variable somewhere this migration does not rewrite.
    Unsupported,
}

/// Whether the script still relies on the variable the session cannot provide.
pub fn needs_migration(contents: &str) -> bool {
    let upper = contents.to_ascii_uppercase();
    upper.contains(OLD) && !upper.contains(PATH_VAR)
}

/// `%let <name> = %sysget(SAS_EXECFILEPATH);` in any spacing or case.
fn assignment_name(line: &str) -> Option<&str> {
    let trimmed = line.trim();
    // Step by characters, not bytes: a Chinese comment can put byte 4 inside a
    // multi-byte character and `&trimmed[..4]` would panic on it.
    if !trimmed.get(..4)?.eq_ignore_ascii_case("%let") {
        return None;
    }
    let (name, value) = trimmed[4..].split_once('=')?;
    let name = name.trim();
    if name.is_empty()
        || !name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
    {
        return None;
    }
    value
        .trim()
        .eq_ignore_ascii_case("%sysget(SAS_EXECFILEPATH);")
        .then_some(name)
}

fn block(name: &str, indent: &str, eol: &str) -> String {
    let inner = format!("{indent}    ");
    [
        format!("{indent}%if %symexist({PATH_VAR}) %then %do;"),
        format!("{inner}%let {name} = &{PATH_VAR};"),
        format!("{indent}%end;"),
        format!("{indent}%else %do;"),
        format!("{inner}%let {name} = %sysget({OLD});"),
        format!("{indent}%end;"),
    ]
    .join(eol)
}

/// Rewrites the known self-location assignments, or explains why it will not.
pub fn migrate(contents: &str) -> Result<String, MigrateError> {
    if !needs_migration(contents) {
        return Err(MigrateError::NothingToDo);
    }
    let eol = if contents.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let mut out = String::with_capacity(contents.len() + 256);
    let mut replaced = 0;
    for line in contents.split_inclusive('\n') {
        let (body, ending) = match line.strip_suffix("\r\n") {
            Some(body) => (body, "\r\n"),
            None => match line.strip_suffix('\n') {
                Some(body) => (body, "\n"),
                None => (line, ""),
            },
        };
        match assignment_name(body) {
            Some(name) => {
                let indent = &body[..body.len() - body.trim_start().len()];
                out.push_str(&block(name, indent, eol));
                out.push_str(ending);
                replaced += 1;
            }
            None => {
                if body.to_ascii_uppercase().contains(OLD) {
                    // Refuse instead of half-fixing the script.
                    return Err(MigrateError::Unsupported);
                }
                out.push_str(line);
            }
        }
    }
    if replaced == 0 {
        return Err(MigrateError::NothingToDo);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

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
