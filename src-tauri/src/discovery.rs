use crate::issue::Issue;
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};
use windows_sys::Win32::Globalization::{CP_ACP, MultiByteToWideChar};

/// One factory (read-only) configuration: the `nls/<code>` directory.
#[derive(Clone, Debug)]
pub struct BuiltIn {
    pub code: String,
    pub cfg: PathBuf,
    pub encoding: Option<String>,
    pub locale: Option<String>,
    pub ui_language: Option<String>,
    pub error: Option<Issue>,
}

#[derive(Clone, Debug)]
pub struct Installation {
    pub exe: PathBuf,
    pub configs: Vec<BuiltIn>,
    /// Directory code of the default configuration, when the root pointer
    /// resolves into `nls/`.
    pub default: Option<String>,
    pub default_error: Option<Issue>,
}

pub fn find_installed() -> Option<PathBuf> {
    ["ProgramW6432", "ProgramFiles", "ProgramFiles(x86)"]
        .into_iter()
        .filter_map(env::var_os)
        .map(|root| PathBuf::from(root).join("SASHome/SASFoundation/9.4/sas.exe"))
        .find_map(|exe| validate_exe(&exe).ok())
}

pub fn validate_exe(exe: &Path) -> Result<PathBuf, Issue> {
    if !exe
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.eq_ignore_ascii_case("sas.exe"))
        || !exe.is_file()
    {
        return Err(Issue::new("invalidExe"));
    }
    dunce::canonicalize(exe).map_err(|error| Issue::with_detail("exeAccess", error))
}

pub fn scan(exe: &Path) -> Result<Vec<BuiltIn>, Issue> {
    let exe = validate_exe(exe)?;
    let nls = exe.parent().unwrap().join("nls");
    let entries = fs::read_dir(&nls).map_err(|error| {
        Issue::with_detail("configDirectory", format!("{}: {error}", nls.display()))
    })?;
    let mut configs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| Issue::with_detail("configEntry", error))?;
        let path = entry.path();
        if !path.is_dir()
            || path
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("sasmisc"))
            || path
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("deployment"))
        {
            continue;
        }
        let cfg = path.join("sasv9.cfg");
        match fs::metadata(&cfg) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            _ => {}
        }
        configs.push(describe(
            &cfg,
            entry.file_name().to_string_lossy().into_owned(),
        ));
    }
    configs.sort_by(|left, right| {
        let (left_prefix, left_number) = split_natural_code(&left.code);
        let (right_prefix, right_number) = split_natural_code(&right.code);
        left_prefix
            .cmp(right_prefix)
            .then_with(|| left_number.cmp(&right_number))
            .then_with(|| left.code.cmp(&right.code))
    });
    if configs.is_empty() {
        return Err(Issue::with_detail("noConfigs", nls.display()));
    }
    Ok(configs)
}

pub fn describe(cfg: &Path, code: String) -> BuiltIn {
    let mut config = BuiltIn {
        code,
        cfg: cfg.to_path_buf(),
        encoding: None,
        locale: None,
        ui_language: None,
        error: None,
    };
    match fs::read(cfg)
        .map_err(|error| Issue::with_detail("configRead", error))
        .and_then(|bytes| decode(&bytes))
    {
        Ok(source) => {
            let options = parse_options(&source);
            config.encoding = options
                .get("encoding")
                .filter(|value| !value.is_empty())
                .cloned();
            config.locale = options.get("locale").cloned();
            config.ui_language = interface_language(&config.code, &source);
            if config.encoding.is_none() {
                config.error = Some(Issue::new("encodingUnknown"));
            }
        }
        Err(error) => config.error = Some(error),
    }
    config
}

fn split_natural_code(code: &str) -> (&str, u64) {
    let split = code
        .find(|ch: char| ch.is_ascii_digit())
        .unwrap_or(code.len());
    let (prefix, suffix) = code.split_at(split);
    (prefix, suffix.parse().unwrap_or(0))
}

pub fn inspect(exe: &Path) -> Result<Installation, Issue> {
    let configs = scan(exe)?;
    let exe = validate_exe(exe)?;
    let root = exe.parent().unwrap();
    let (default, default_error) = match fs::read(root.join("sasv9.cfg"))
        .map_err(|error| Issue::with_detail("rootConfigRead", error))
        .and_then(|bytes| decode(&bytes))
    {
        Ok(source) => {
            let options = parse_options(&source);
            match options
                .get("config")
                .map(|value| value.trim())
                .filter(|value| !value.is_empty())
            {
                Some(value) => match resolve_root_reference(root, value) {
                    Ok(path) => match configs.iter().find(|config| {
                        dunce::canonicalize(&config.cfg)
                            .is_ok_and(|cfg| windows_path_eq(&cfg, &path))
                    }) {
                        Some(config) => (Some(config.code.clone()), None),
                        None => (None, Some(Issue::new("defaultConfigUnavailable"))),
                    },
                    Err(error) => (None, Some(error)),
                },
                None => (None, Some(Issue::new("defaultConfigPointerMissing"))),
            }
        }
        Err(error) => (None, Some(error)),
    };
    Ok(Installation {
        exe,
        configs,
        default,
        default_error,
    })
}

/// `<exe dir>/nls/<code>/sasv9.cfg`, canonicalized.
pub fn factory_cfg_for(exe: &Path, code: &str) -> Result<PathBuf, Issue> {
    if code.is_empty() || code.contains('\\') || code.contains('/') || code == "." || code == ".." {
        return Err(Issue::with_detail("configUnavailable", code));
    }
    let exe = validate_exe(exe)?;
    let cfg = exe
        .parent()
        .unwrap()
        .join("nls")
        .join(code)
        .join("sasv9.cfg");
    if !cfg.is_file() {
        return Err(Issue::with_detail("configUnavailable", code));
    }
    dunce::canonicalize(&cfg).map_err(|error| Issue::with_detail("configUnavailable", error))
}

/// The factory cfg the SAS root `sasv9.cfg` points at. It may live outside
/// `nls/` (for example an installation-managed shared file).
pub fn resolved_default_cfg(exe: &Path) -> Result<PathBuf, Issue> {
    let exe = validate_exe(exe)?;
    let root = exe.parent().unwrap();
    let bytes = fs::read(root.join("sasv9.cfg"))
        .map_err(|error| Issue::with_detail("rootConfigRead", error))?;
    let source = decode(&bytes)?;
    let options = parse_options(&source);
    let value = options
        .get("config")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| Issue::new("defaultConfigPointerMissing"))?;
    resolve_root_reference(root, value)
}

fn resolve_root_reference(root: &Path, value: &str) -> Result<PathBuf, Issue> {
    let normalized = value.replace('/', "\\");
    let path = if let Some((prefix, tail)) = normalized.split_once('\\')
        && prefix.eq_ignore_ascii_case("!sasroot")
    {
        root.join(tail)
    } else if let Some(value) = normalized.strip_prefix('!') {
        let (variable, tail) = value.split_once('\\').unwrap_or((value, ""));
        let base = match variable.to_ascii_uppercase().as_str() {
            "SASROOT" => root.to_path_buf(),
            "SAS_HOME" => root
                .parent()
                .and_then(Path::parent)
                .unwrap_or(root)
                .to_path_buf(),
            _ => env::var_os(variable).map(PathBuf::from).ok_or_else(|| {
                Issue::with_detail(
                    "defaultConfigUnavailable",
                    format!("Unknown variable !{variable}"),
                )
            })?,
        };
        base.join(tail)
    } else {
        let path = PathBuf::from(&normalized);
        if path.is_absolute() {
            path
        } else {
            root.join(path)
        }
    };
    dunce::canonicalize(&path)
        .map_err(|error| Issue::with_detail("defaultConfigUnavailable", error))
}

fn decode(bytes: &[u8]) -> Result<String, Issue> {
    if let Some(data) = bytes
        .strip_prefix(&[0xff, 0xfe])
        .or_else(|| bytes.strip_prefix(&[0xfe, 0xff]))
    {
        let little_endian = bytes[0] == 0xff;
        let (words, remainder) = data.as_chunks::<2>();
        if !remainder.is_empty() {
            return Err(Issue::new("utf16Incomplete"));
        }
        let words: Vec<_> = words
            .iter()
            .map(|&pair| {
                if little_endian {
                    u16::from_le_bytes(pair)
                } else {
                    u16::from_be_bytes(pair)
                }
            })
            .collect();
        return String::from_utf16(&words).map_err(|_| Issue::new("configDecode"));
    }
    if let Ok(value) = std::str::from_utf8(bytes) {
        return Ok(value.into());
    }

    let input_len = i32::try_from(bytes.len()).map_err(|_| Issue::new("configTooLarge"))?;
    // Windows SAS configuration files can also use the current ANSI code page.
    let len = unsafe {
        MultiByteToWideChar(
            CP_ACP,
            0,
            bytes.as_ptr(),
            input_len,
            std::ptr::null_mut(),
            0,
        )
    };
    if len > 0 {
        let mut words = vec![0; len as usize];
        let written = unsafe {
            MultiByteToWideChar(
                CP_ACP,
                0,
                bytes.as_ptr(),
                input_len,
                words.as_mut_ptr(),
                len,
            )
        };
        if written == len {
            return Ok(String::from_utf16_lossy(&words));
        }
    }
    Err(Issue::new("configDecode"))
}

fn configuration_lines(source: &str) -> impl Iterator<Item = String> + '_ {
    let mut in_comment = false;
    source
        .trim_start_matches('\u{feff}')
        .lines()
        .map(move |line| {
            let mut clean = String::new();
            let mut quote = None;
            let mut chars = line.chars().peekable();
            while let Some(ch) = chars.next() {
                if in_comment {
                    if ch == '*' && chars.next_if_eq(&'/').is_some() {
                        in_comment = false;
                    }
                } else if quote.is_none() && ch == '/' && chars.next_if_eq(&'*').is_some() {
                    in_comment = true;
                } else {
                    if quote == Some(ch) {
                        quote = None;
                    } else if quote.is_none() && matches!(ch, '"' | '\'') {
                        quote = Some(ch);
                    }
                    clean.push(ch);
                }
            }
            clean
        })
}

fn parse_options(source: &str) -> HashMap<String, String> {
    let mut options = HashMap::new();
    for clean in configuration_lines(source) {
        let Some(option) = clean.trim_start().strip_prefix('-') else {
            continue;
        };
        let Some((name, value)) = option.split_once(|ch: char| ch.is_whitespace() || ch == '=')
        else {
            continue;
        };
        let name = name.to_ascii_lowercase();
        if !matches!(
            name.as_str(),
            "encoding" | "locale" | "sasuser" | "config" | "resourcesloc" | "msg"
        ) {
            continue;
        }
        let value =
            value.trim_start_matches(|ch: char| ch.is_whitespace() || matches!(ch, '=' | '('));
        let value = if let Some(quote) = value.chars().next().filter(|ch| matches!(ch, '"' | '\''))
        {
            value[1..].split_once(quote).map(|(value, _)| value)
        } else {
            value.split_whitespace().next()
        };
        if let Some(value) = value {
            options.insert(name, value.trim().into());
        }
    }
    options
}

fn windows_path_eq(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .eq_ignore_ascii_case(&right.to_string_lossy())
}

fn localized_directory(code: &str) -> Option<&'static str> {
    match code {
        "zh" => Some("zh-Hans"),
        "zt" => Some("zh-Hant"),
        "ja" => Some("ja"),
        "ko" => Some("ko"),
        "en" | "1d" => Some("en"),
        "fr" => Some("fr"),
        "de" => Some("de"),
        "es" => Some("es"),
        "it" => Some("it"),
        "ar" => Some("ar"),
        "da" => Some("da"),
        "nl" => Some("nl"),
        "no" => Some("no"),
        "pb" => Some("pt-BR"),
        "pt" => Some("pt"),
        "ru" => Some("ru"),
        "cs" => Some("cs"),
        "hu" => Some("hu"),
        "pl" => Some("pl"),
        "sv" => Some("sv"),
        "iw" | "he" => Some("he"),
        "tr" => Some("tr"),
        _ => None,
    }
}

fn interface_language(code: &str, source: &str) -> Option<String> {
    let mut resources = false;
    let mut core_resources = false;
    for line in configuration_lines(source) {
        let line = line.trim().to_ascii_lowercase().replace('/', "\\");
        if line.starts_with('-') {
            resources = line
                .strip_prefix('-')
                .and_then(|line| line.split(|c: char| c.is_whitespace() || c == '=').next())
                .is_some_and(|option| matches!(option, "resourcesloc" | "msg"));
        }
        if resources {
            if let Some((_, location)) = line.split_once("\\nls\\")
                && let Some((language, _)) = location.split_once('\\')
                && let Some(language) = localized_directory(language)
            {
                return Some(language.into());
            }
            core_resources |= line.contains("\\core\\resource") || line.contains("\\core\\sasmsg");
        }
    }
    core_resources
        .then_some("en")
        .or_else(|| localized_directory(&code.to_ascii_lowercase()))
        .map(String::from)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(nls: &[(&str, &str)], root_cfg: Option<&str>) -> (tempfile::TempDir, PathBuf) {
        let temp = tempfile::tempdir().unwrap();
        let exe = temp.path().join("sas.exe");
        fs::write(&exe, []).unwrap();
        for (code, source) in nls {
            let directory = temp.path().join("nls").join(code);
            fs::create_dir_all(&directory).unwrap();
            fs::write(directory.join("sasv9.cfg"), source).unwrap();
        }
        if let Some(root_cfg) = root_cfg {
            fs::write(temp.path().join("sasv9.cfg"), root_cfg).unwrap();
        }
        (temp, exe)
    }

    #[test]
    fn commented_resource_locations_do_not_affect_the_sas_interface_language() {
        let config = r#"/*
-RESOURCESLOC "!SASROOT\nls\zh\resource"
*/
-RESOURCESLOC "!SASROOT\core\resource""#;
        assert_eq!(interface_language("u8", config).as_deref(), Some("en"));
    }

    #[test]
    fn latest_matching_option_wins_and_comments_are_ignored() {
        let cfg = "\u{feff}/*\n-ENCODING fake\n*/\n -encoding wlatin1\n-ENCODING \"UTF-8\"\n-LOCALE zh_CN\n-SASUSER \"?FOLDERID_Documents\\My SAS Files\\9.4\"";
        let options = parse_options(cfg);
        assert_eq!(options.get("encoding").map(String::as_str), Some("UTF-8"));
        assert_eq!(options.get("locale").map(String::as_str), Some("zh_CN"));
        assert_eq!(
            options.get("sasuser").map(String::as_str),
            Some("?FOLDERID_Documents\\My SAS Files\\9.4")
        );
    }

    #[test]
    fn installation_scan_excludes_deployment_cfg_and_uses_numeric_locale_sort() {
        let (_temp, exe) = fixture(
            &[
                ("aa10", "-ENCODING wlatin1\n-LOCALE en_US"),
                ("aa2", "-ENCODING wlatin1\n-LOCALE en_US"),
            ],
            Some(r#"-CONFIG "!SASROOT\nls\aa2\sasv9.cfg""#),
        );
        let deployment = exe
            .parent()
            .unwrap()
            .join("nls/aa1/sasmisc/deployment/sasv9.cfg");
        fs::create_dir_all(deployment.parent().unwrap()).unwrap();
        fs::write(deployment, "-ENCODING invalid\n-LOCALE zz_ZZ").unwrap();

        let installation = inspect(&exe).unwrap();

        assert_eq!(installation.configs.len(), 2);
        assert_eq!(installation.configs[0].code, "aa2");
        assert_eq!(installation.configs[1].code, "aa10");
        assert_eq!(installation.default.as_deref(), Some("aa2"));
    }

    #[test]
    fn separates_configs_with_same_encoding_and_reports_incomplete_config() {
        let (_temp, exe) = fixture(
            &[
                ("zh", "-encoding euc-cn"),
                ("1d", "-encoding euc-cn"),
                ("bad", "-locale zh_CN"),
            ],
            None,
        );
        let configs = scan(&exe).unwrap();
        assert_eq!(configs.len(), 3);
        assert_eq!(configs[0].code, "1d");
        assert_eq!(configs[2].code, "zh");
        assert_ne!(configs[0].cfg, configs[2].cfg);
        assert_eq!(configs[0].encoding.as_deref(), Some("euc-cn"));
        assert!(configs[1].error.is_some());
    }

    #[test]
    fn identifies_the_sas_interface_language_from_resource_locations() {
        let (_temp, exe) = fixture(
            &[
                (
                    "zh",
                    "-ENCODING EUC-CN\n-LOCALE zh_CN\n-RESOURCESLOC \"!SASROOT\\nls\\zh\\resource\"",
                ),
                (
                    "1d",
                    "-ENCODING EUC-CN\n-LOCALE zh_CN\n-RESOURCESLOC \"!SASROOT\\core\\resource\"",
                ),
            ],
            Some(r#"-CONFIG "!SASROOT\nls\zh\sasv9.cfg""#),
        );
        let installation = inspect(&exe).unwrap();
        assert_eq!(installation.configs[0].ui_language.as_deref(), Some("en"));
        assert_eq!(
            installation.configs[1].ui_language.as_deref(),
            Some("zh-Hans")
        );
        assert_eq!(installation.default.as_deref(), Some("zh"));
    }

    #[test]
    fn reports_an_unresolvable_root_default_pointer_instead_of_silently_falling_back() {
        let (_temp, exe) = fixture(
            &[("en", "-ENCODING wlatin1\n-LOCALE en_US")],
            Some(r#"-CONFIG "!SASROOT\\nls\\missing\\sasv9.cfg""#),
        );
        let installation = inspect(&exe).unwrap();
        assert_eq!(installation.default, None);
        assert_eq!(
            installation.default_error.as_ref().map(|error| error.code),
            Some("defaultConfigUnavailable")
        );
    }

    #[test]
    fn resolves_default_pointer_with_environment_paths_and_absolute_targets() {
        let temp = tempfile::tempdir().unwrap();
        let sas_home = temp.path().join("SASHome");
        let root = sas_home.join("SASFoundation/9.4");
        let sas_root_cfg = root.join("nls/zh/sasv9.cfg");
        let sas_home_cfg = root.join("nls/en/sasv9.cfg");
        fs::create_dir_all(sas_root_cfg.parent().unwrap()).unwrap();
        fs::create_dir_all(sas_home_cfg.parent().unwrap()).unwrap();
        fs::write(&sas_root_cfg, b"fixture").unwrap();
        fs::write(&sas_home_cfg, b"fixture").unwrap();
        assert!(windows_path_eq(
            &resolve_root_reference(&root, r"!SASROOT\nls\zh\sasv9.cfg").unwrap(),
            &dunce::canonicalize(sas_root_cfg).unwrap()
        ));
        assert!(windows_path_eq(
            &resolve_root_reference(&root, r"!SAS_HOME\SASFoundation\9.4\nls\en\sasv9.cfg")
                .unwrap(),
            &dunce::canonicalize(sas_home_cfg).unwrap()
        ));
    }

    #[test]
    fn resolved_default_cfg_reads_the_root_pointer_target_outside_nls() {
        let temp = tempfile::tempdir().unwrap();
        let exe = temp.path().join("sas.exe");
        fs::write(&exe, []).unwrap();
        let shared = temp.path().join("shared/sasv9.cfg");
        fs::create_dir_all(shared.parent().unwrap()).unwrap();
        fs::write(&shared, "-ENCODING wlatin1").unwrap();
        fs::write(
            temp.path().join("sasv9.cfg"),
            "-CONFIG \"!SASROOT\\shared\\sasv9.cfg\"",
        )
        .unwrap();

        assert!(windows_path_eq(
            &resolved_default_cfg(&exe).unwrap(),
            &dunce::canonicalize(&shared).unwrap()
        ));
    }

    #[test]
    fn factory_cfg_for_requires_an_existing_nls_directory() {
        let (_temp, exe) = fixture(&[("u8", "-ENCODING UTF-8")], None);
        assert!(factory_cfg_for(&exe, "u8").is_ok());
        assert_eq!(
            factory_cfg_for(&exe, "missing").unwrap_err().code,
            "configUnavailable"
        );
        assert_eq!(
            factory_cfg_for(&exe, "../escape").unwrap_err().code,
            "configUnavailable"
        );
    }
}
