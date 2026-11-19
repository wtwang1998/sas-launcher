use crate::issue::Issue;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env, fs,
    path::{Path, PathBuf},
};
use windows_sys::Win32::Globalization::{CP_ACP, MultiByteToWideChar};

#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub struct ConfigKey {
    pub exe: PathBuf,
    pub cfg: PathBuf,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LaunchConfig {
    pub key: ConfigKey,
    pub code: String,
    pub custom_id: Option<uuid::Uuid>,
    pub custom_name: Option<String>,
    pub base: Option<ConfigKey>,
    pub file: Option<PathBuf>,
    pub encoding: Option<String>,
    pub locale: Option<String>,
    pub default_profile: Option<PathBuf>,
    pub ui_language: Option<String>,
    pub error: Option<Issue>,
}

impl LaunchConfig {
    pub fn cfg_path(&self) -> &Path {
        self.file.as_deref().unwrap_or(&self.key.cfg)
    }
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

pub fn scan(exe: &Path, documents: Option<&Path>) -> Result<Vec<LaunchConfig>, Issue> {
    let exe = validate_exe(exe)?;
    let nls = exe.parent().unwrap().join("nls");
    let entries = fs::read_dir(&nls).map_err(|error| {
        Issue::with_detail("configDirectory", format!("{}: {error}", nls.display()))
    })?;
    let mut configs = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| Issue::with_detail("configEntry", error))?;
        if !entry.path().is_dir() {
            continue;
        }
        let cfg = entry.path().join("sasv9.cfg");
        match fs::metadata(&cfg) {
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            _ => {}
        }
        configs.push(describe(
            ConfigKey {
                exe: exe.clone(),
                cfg,
            },
            entry.file_name().to_string_lossy().into_owned(),
            documents,
        ));
    }
    configs.sort_by(|left, right| left.code.cmp(&right.code));
    if configs.is_empty() {
        return Err(Issue::with_detail("noConfigs", nls.display()));
    }
    Ok(configs)
}

pub fn describe(key: ConfigKey, code: String, documents: Option<&Path>) -> LaunchConfig {
    let result = fs::read(&key.cfg)
        .map_err(|error| Issue::with_detail("configRead", error))
        .and_then(|bytes| decode(&bytes));
    let mut config = LaunchConfig {
        key,
        code,
        custom_id: None,
        custom_name: None,
        base: None,
        file: None,
        encoding: None,
        locale: None,
        default_profile: None,
        ui_language: None,
        error: None,
    };
    match result {
        Ok(source) => {
            let options = parse_options(&source);
            config.encoding = options
                .get("encoding")
                .filter(|value| !value.is_empty())
                .cloned();
            config.locale = options.get("locale").cloned();
            config.ui_language = interface_language(&config.code, &source);
            config.default_profile = options
                .get("sasuser")
                .and_then(|value| resolve_profile(value, documents));
            if config.encoding.is_none() {
                config.error = Some(Issue::new("encodingUnknown"));
            }
        }
        Err(error) => config.error = Some(error),
    }
    config
}

fn resolve_profile(value: &str, documents: Option<&Path>) -> Option<PathBuf> {
    if let Some(rest) = value.strip_prefix('?') {
        let (folder, tail) = rest.split_once(['\\', '/']).unwrap_or((rest, ""));
        return if folder.eq_ignore_ascii_case("FOLDERID_Documents") {
            documents.map(|path| path.join(tail))
        } else {
            None
        };
    }
    if let Some(rest) = value.strip_prefix('!') {
        let (variable, tail) = rest.split_once(['\\', '/']).unwrap_or((rest, ""));
        return Some(PathBuf::from(env::var_os(variable)?).join(tail));
    }
    let path = PathBuf::from(value);
    path.is_absolute().then_some(path)
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
        if !matches!(name.as_str(), "encoding" | "locale" | "sasuser" | "config") {
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

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileCandidate {
    pub directory: PathBuf,
    pub files: Vec<&'static str>,
    pub keys: Vec<ConfigKey>,
}

pub struct Installation {
    pub configs: Vec<LaunchConfig>,
    pub default_key: Option<ConfigKey>,
    pub sources: Vec<ProfileCandidate>,
}

pub fn inspect(exe: &Path, documents: Option<&Path>) -> Result<Installation, Issue> {
    let configs = scan(exe, documents)?;
    let root = configs[0].key.exe.parent().unwrap();
    let root_options = fs::read(root.join("sasv9.cfg"))
        .ok()
        .and_then(|bytes| decode(&bytes).ok())
        .map(|source| parse_options(&source))
        .unwrap_or_default();
    let default_key = root_options
        .get("config")
        .and_then(|value| {
            let normalized = value.replace('/', "\\");
            if let Some((prefix, tail)) = normalized.split_once('\\')
                && prefix.eq_ignore_ascii_case("!sasroot")
            {
                return dunce::canonicalize(root.join(tail)).ok();
            }
            dunce::canonicalize(root.join(normalized)).ok()
        })
        .and_then(|default| {
            configs.iter().find(|config| {
                config.error.is_none()
                    && dunce::canonicalize(&config.key.cfg)
                        .is_ok_and(|cfg| windows_path_eq(&cfg, &default))
            })
        })
        .map(|config| config.key.clone());

    let mut sources: Vec<ProfileCandidate> = Vec::new();
    if let Some(directory) = root_options
        .get("sasuser")
        .and_then(|value| resolve_profile(value, documents))
    {
        add_profile_candidate(&mut sources, &directory, default_key.as_ref());
    }
    for config in &configs {
        if let Some(directory) = &config.default_profile {
            add_profile_candidate(&mut sources, directory, Some(&config.key));
        }
    }
    sources.sort_by_key(|source| {
        !default_key
            .as_ref()
            .is_some_and(|key| source.keys.contains(key))
    });
    Ok(Installation {
        configs,
        default_key,
        sources,
    })
}

pub fn add_profile_candidate(
    sources: &mut Vec<ProfileCandidate>,
    path: &Path,
    key: Option<&ConfigKey>,
) {
    let Ok(directory) = dunce::canonicalize(path) else {
        return;
    };
    if let Some(candidate) = sources
        .iter_mut()
        .find(|candidate| windows_path_eq(&candidate.directory, &directory))
    {
        if let Some(key) = key
            && !candidate.keys.contains(key)
        {
            candidate.keys.push(key.clone());
        }
        return;
    }
    let files: Vec<_> = crate::profile::FILES
        .iter()
        .copied()
        .filter(|name| directory.join(name).is_file())
        .collect();
    if !files.is_empty() {
        sources.push(ProfileCandidate {
            directory,
            files,
            keys: key.cloned().into_iter().collect(),
        });
    }
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
    fn separates_configs_with_same_encoding_and_reports_incomplete_config() {
        let root = tempfile::tempdir().unwrap();
        let exe = root.path().join("sas.exe");
        fs::write(&exe, []).unwrap();
        for (code, source) in [
            ("zh", "-encoding euc-cn"),
            ("1d", "-encoding euc-cn"),
            ("bad", "-locale zh_CN"),
        ] {
            let dir = root.path().join("nls").join(code);
            fs::create_dir_all(&dir).unwrap();
            fs::write(dir.join("sasv9.cfg"), source).unwrap();
        }
        let choices = scan(&exe, None).unwrap();
        assert_eq!(choices.len(), 3);
        assert_eq!(choices[0].code, "1d");
        assert_eq!(choices[2].code, "zh");
        assert_ne!(choices[0].key, choices[2].key);
        assert_eq!(choices[0].encoding.as_deref(), Some("euc-cn"));
        assert!(choices[1].error.is_some());
    }

    #[test]
    fn discovers_one_candidate_for_shared_profile_and_distinguishes_sas_resources() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        let exe = root.join("sas.exe");
        fs::write(&exe, []).unwrap();
        let user = root.join("User Files/9.4");
        fs::create_dir_all(&user).unwrap();
        fs::write(user.join("profile.sas7bcat"), b"test settings").unwrap();
        fs::write(
            root.join("sasv9.cfg"),
            r#"-CONFIG "!SASROOT\nls\zh\sasv9.cfg""#,
        )
        .unwrap();
        for (code, resources) in [
            ("zh", r"!SASROOT\nls\zh\resource"),
            ("1d", r"!SASROOT\core\resource"),
        ] {
            let path = root.join("nls").join(code);
            fs::create_dir_all(&path).unwrap();
            fs::write(path.join("sasv9.cfg"), format!(
                "-ENCODING EUC-CN\n-LOCALE zh_CN\n-SASUSER \"?FOLDERID_Documents\\User Files\\9.4\"\n-RESOURCESLOC (\n\"{resources}\"\n)"
            )).unwrap();
        }
        let result = inspect(&exe, Some(root)).unwrap();
        assert_eq!(result.configs[0].ui_language.as_deref(), Some("en"));
        assert_eq!(result.configs[1].ui_language.as_deref(), Some("zh-Hans"));
        assert_eq!(result.default_key.as_ref(), Some(&result.configs[1].key));
        assert_eq!(result.sources.len(), 1);
        assert_eq!(result.sources[0].keys.len(), 2);
        assert_eq!(result.sources[0].files, vec!["profile.sas7bcat"]);
        fs::remove_file(user.join("profile.sas7bcat")).unwrap();
        assert!(inspect(&exe, Some(root)).unwrap().sources.is_empty());
    }

    #[test]
    fn resolves_documents_using_windows_known_folder() {
        assert_eq!(
            resolve_profile(
                r"?FOLDERID_Documents\My SAS Files\9.4",
                Some(Path::new(r"D:\Redirected Documents"))
            ),
            Some(PathBuf::from(r"D:\Redirected Documents\My SAS Files\9.4"))
        );
    }
}
