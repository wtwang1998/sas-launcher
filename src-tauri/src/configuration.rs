use crate::{
    discovery::BuiltIn,
    issue::Issue,
    store::{ConfigRef, Preset, UserConfig},
    view::ConfigView,
};
use serde::Serialize;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

const DEFAULT_NAME_PREFIX: &str = "用户配置（衍生自 ";
pub const DEFAULT_BASE_LABEL: &str = "默认配置";

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum PresetStatus {
    Verified,
    Failed,
    Unverified,
}

pub fn default_config_name(base_label: &str, existing: &[UserConfig]) -> String {
    let prefix = format!("{DEFAULT_NAME_PREFIX}{base_label}）#");
    let mut used = Vec::new();
    for config in existing {
        if let Some(rest) = config.name.strip_prefix(&prefix)
            && let Ok(number) = rest.parse::<u32>()
            && number > 0
        {
            used.push(number);
        }
    }
    let mut number = 1;
    while used.contains(&number) {
        number += 1;
    }
    format!("{prefix}{number}")
}

/// Exactly three CRLF lines, the last one newline-terminated. The factory cfg
/// text is never copied: the thin cfg only points at it.
pub fn write_thin_cfg(
    path: &Path,
    factory: &Path,
    encoding: &str,
    locale: &str,
) -> Result<(), Issue> {
    let factory = absolute(factory);
    let mut text = String::new();
    text.push_str("-CONFIG \"");
    text.push_str(&factory.to_string_lossy());
    text.push_str("\"\r\n");
    text.push_str("-ENCODING ");
    text.push_str(&encoding.trim().to_ascii_lowercase());
    text.push_str("\r\n");
    text.push_str("-LOCALE ");
    text.push_str(locale.trim());
    text.push_str("\r\n");
    let mut file =
        fs::File::create(path).map_err(|error| Issue::with_detail("configurationWrite", error))?;
    file.write_all(text.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|error| Issue::with_detail("configurationWrite", error))
}

fn absolute(path: &Path) -> PathBuf {
    dunce::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

pub fn validate_options(encoding: &str, locale: &str) -> Result<(), Issue> {
    let encoding = encoding.trim();
    let locale = locale.trim();
    if encoding.is_empty() || locale.is_empty() {
        return Err(Issue::new("invalidLocaleEncoding"));
    }
    let bytes = encoding.as_bytes();
    let encoding_ok = bytes.len() <= 32
        && bytes[0].is_ascii_alphanumeric()
        && bytes
            .iter()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'));
    if !encoding_ok || !locale_ok(locale) {
        return Err(Issue::new("invalidLocaleEncoding"));
    }
    Ok(())
}

fn locale_ok(locale: &str) -> bool {
    if locale.is_empty() || locale.chars().count() > 32 {
        return false;
    }
    if !locale
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
        || !locale.starts_with(|ch: char| ch.is_ascii_alphabetic())
    {
        return false;
    }
    // SAS accepts either a code (`en_US`, `ja_JP`) or a bare language name
    // (`Spanish`, `German`) — the factory configurations use both.
    let parts: Vec<&str> = locale.split(['_', '-']).collect();
    match parts.as_slice() {
        [language] => (3..=20).contains(&language.chars().count()),
        [language, region] => {
            (2..=3).contains(&language.chars().count()) && region.chars().count() == 2
        }
        _ => false,
    }
}

fn normalize(value: &str) -> String {
    value.trim().replace('-', "_").to_ascii_lowercase()
}

pub fn is_factory_combo(factory: &[BuiltIn], encoding: &str, locale: &str) -> bool {
    let encoding = normalize(encoding);
    let locale = normalize(locale);
    factory.iter().any(|built_in| {
        built_in
            .encoding
            .as_deref()
            .is_some_and(|value| normalize(value) == encoding)
            && built_in
                .locale
                .as_deref()
                .is_some_and(|value| normalize(value) == locale)
    })
}

pub fn preset_status(
    presets: &[Preset],
    factory: &[BuiltIn],
    encoding: &str,
    locale: &str,
) -> PresetStatus {
    if encoding.trim().is_empty() || locale.trim().is_empty() {
        return PresetStatus::Unverified;
    }
    if is_factory_combo(factory, encoding, locale) {
        return PresetStatus::Verified;
    }
    let encoding = normalize(encoding);
    let locale = normalize(locale);
    match presets.iter().rev().find(|preset| {
        normalize(&preset.encoding) == encoding && normalize(&preset.locale) == locale
    }) {
        Some(preset) if preset.ok => PresetStatus::Verified,
        Some(_) => PresetStatus::Failed,
        None => PresetStatus::Unverified,
    }
}

/// Insert or update one remembered combination. Factory combinations are not
/// stored by the caller (see `is_factory_combo`).
pub fn record_launch(presets: &mut Vec<Preset>, encoding: &str, locale: &str, ok: bool) {
    let encoding = encoding.trim();
    let locale = locale.trim();
    if encoding.is_empty() || locale.is_empty() {
        return;
    }
    let same = |preset: &Preset| {
        normalize(&preset.encoding) == normalize(encoding)
            && normalize(&preset.locale) == normalize(locale)
    };
    if let Some(preset) = presets.iter_mut().find(|preset| same(preset)) {
        preset.ok = ok;
        preset.encoding = encoding.to_owned();
        preset.locale = locale.to_owned();
        return;
    }
    presets.push(Preset {
        encoding: encoding.to_owned(),
        locale: locale.to_owned(),
        ok,
    });
}

pub fn built_in_view(built_in: &BuiltIn, presets: &[Preset]) -> ConfigView {
    ConfigView {
        reference: ConfigRef::BuiltIn {
            code: built_in.code.clone(),
        },
        name: built_in.code.clone(),
        encoding: built_in.encoding.clone(),
        locale: built_in.locale.clone(),
        ui_language: built_in.ui_language.clone(),
        preset: match (&built_in.encoding, &built_in.locale) {
            (Some(encoding), Some(locale)) => {
                preset_status(presets, std::slice::from_ref(built_in), encoding, locale)
            }
            _ => PresetStatus::Unverified,
        },
        derived_from: None,
        has_profile: false,
        error: built_in.error.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{discovery, store::ConfigBase};
    use uuid::Uuid;

    fn built_in(code: &str, encoding: Option<&str>, locale: Option<&str>) -> BuiltIn {
        BuiltIn {
            code: code.into(),
            cfg: PathBuf::from(format!("C:\\SAS\\nls\\{code}\\sasv9.cfg")),
            encoding: encoding.map(String::from),
            locale: locale.map(String::from),
            ui_language: None,
            error: None,
        }
    }

    #[test]
    fn built_in_rows_use_the_directory_code_and_factory_preset_is_verified() {
        let temp = tempfile::tempdir().unwrap();
        let exe = temp.path().join("sas.exe");
        fs::write(&exe, []).unwrap();
        for (code, encoding, locale) in [
            ("zh", "EUC-CN", "zh_CN"),
            ("u8", "UTF-8", "zh_CN"),
            ("en", "wlatin1", "en_US"),
        ] {
            let directory = temp.path().join("nls").join(code);
            fs::create_dir_all(&directory).unwrap();
            fs::write(
                directory.join("sasv9.cfg"),
                format!("-ENCODING {encoding}\r\n-LOCALE {locale}\r\n"),
            )
            .unwrap();
        }
        fs::write(
            temp.path().join("sasv9.cfg"),
            "-CONFIG \"!SASROOT\\nls\\zh\\sasv9.cfg\"",
        )
        .unwrap();
        let configs = discovery::scan(&exe).unwrap();
        let presets = Vec::new();
        let rows: Vec<ConfigView> = configs
            .iter()
            .map(|built_in| built_in_view(built_in, &presets))
            .collect();
        for code in ["zh", "u8", "en"] {
            let row = rows
                .iter()
                .find(|row| row.reference == ConfigRef::BuiltIn { code: code.into() })
                .unwrap_or_else(|| panic!("missing row {code}"));
            assert_eq!(row.name, code);
            assert_eq!(row.derived_from, None);
            assert!(!row.has_profile);
        }
        let factory = &configs;
        assert_eq!(
            preset_status(&presets, factory, "EUC-CN", "zh_CN"),
            PresetStatus::Verified
        );
        assert_eq!(
            preset_status(&presets, factory, "UTF-8", "zh_CN"),
            PresetStatus::Verified
        );
        assert_eq!(
            preset_status(&presets, factory, "UTF-8", "en_US"),
            PresetStatus::Unverified
        );
        assert_eq!(
            serde_json::to_value(PresetStatus::Verified).unwrap(),
            "verified"
        );
        assert_eq!(
            serde_json::to_value(PresetStatus::Failed).unwrap(),
            "failed"
        );
        assert_eq!(
            serde_json::to_value(PresetStatus::Unverified).unwrap(),
            "unverified"
        );
    }

    #[test]
    fn thin_cfg_has_three_lines_and_no_copy_of_factory_text() {
        let temp = tempfile::tempdir().unwrap();
        let factory = temp.path().join("nls").join("u8").join("sasv9.cfg");
        fs::create_dir_all(factory.parent().unwrap()).unwrap();
        fs::write(
            &factory,
            "-ENCODING UTF-8\r\n-LOCALE zh_CN\r\n-SASUSER \"?FOLDERID_Documents\\My SAS Files\\9.4\"\r\n-LOGCONFIG huge settings block\r\n",
        )
        .unwrap();
        let thin = temp.path().join("thin").join("sasv9.cfg");
        fs::create_dir_all(thin.parent().unwrap()).unwrap();

        write_thin_cfg(&thin, &factory, "UTF-8", "zh_CN").unwrap();

        let text = fs::read_to_string(&thin).unwrap();
        let expected_factory = dunce::canonicalize(&factory).unwrap();
        assert_eq!(
            text,
            format!(
                "-CONFIG \"{}\"\r\n-ENCODING utf-8\r\n-LOCALE zh_CN\r\n",
                expected_factory.display()
            )
        );
        assert_eq!(text.lines().count(), 3);
        assert!(text.ends_with("\r\n"));
        assert!(!text.contains("LOGCONFIG"));
        assert!(!text.contains("-SASUSER"));
    }

    #[test]
    fn accepts_codes_and_bare_language_names_but_rejects_garbage() {
        for (encoding, locale) in [
            ("wlatin1", "en_US"),
            ("UTF-8", "zh_CN"),
            ("wlatin1", "Spanish"),
            ("wcyrillic", "Russian"),
        ] {
            assert!(
                validate_options(encoding, locale).is_ok(),
                "{encoding}/{locale}"
            );
        }
        for (encoding, locale) in [
            ("", "zh_CN"),
            ("wlatin1", ""),
            ("wlatin1", "两个词"),
            ("wlatin1", "Spanish Spanish"),
            ("wlatin1", "with space"),
            ("wla tin1", "en_US"),
        ] {
            assert!(
                validate_options(encoding, locale).is_err(),
                "{encoding}/{locale} must be rejected"
            );
        }
    }

    #[test]
    fn default_name_numbers_per_source() {
        let make = |name: &str| UserConfig {
            id: Uuid::new_v4(),
            name: name.into(),
            base: ConfigBase::BuiltIn { code: "u8".into() },
            encoding: "UTF-8".into(),
            locale: "zh_CN".into(),
            revision: Uuid::new_v4(),
            profile_revision: None,
            factory: PathBuf::from("sasv9.cfg"),
        };
        assert_eq!(default_config_name("u8", &[]), "用户配置（衍生自 u8）#1");
        let existing = vec![
            make("用户配置（衍生自 u8）#1"),
            make("用户配置（衍生自 u8）#2"),
            make("别的名字"),
        ];
        assert_eq!(
            default_config_name("u8", &existing),
            "用户配置（衍生自 u8）#3"
        );
        let gap = vec![
            make("用户配置（衍生自 u8）#1"),
            make("用户配置（衍生自 u8）#3"),
        ];
        assert_eq!(default_config_name("u8", &gap), "用户配置（衍生自 u8）#2");
        // A different source restarts at #1.
        assert_eq!(
            default_config_name("zh", &existing),
            "用户配置（衍生自 zh）#1"
        );
    }

    #[test]
    fn validates_locale_and_encoding_pairs() {
        assert!(validate_options("UTF-8", "zh_CN").is_ok());
        assert!(validate_options("euc-cn", "en-US").is_ok());
        assert!(validate_options("wlatin1", "zh").is_err());
        assert!(validate_options("", "zh_CN").is_err());
        assert!(validate_options("UTF-8", "").is_err());
        assert!(validate_options("UTF 8", "zh_CN").is_err());
        assert!(validate_options("UTF-8", "zh_CN_extra").is_err());
        assert!(validate_options("UTF-8", "z_CN").is_err());
        assert!(validate_options(&"x".repeat(33), "zh_CN").is_err());
    }

    #[test]
    fn remembers_non_factory_combinations_only() {
        let factory = vec![built_in("u8", Some("UTF-8"), Some("zh_CN"))];
        let mut presets = Vec::new();
        assert!(is_factory_combo(&factory, "utf_8", "ZH-cn"));
        record_launch(&mut presets, "EUC-CN", "ja_JP", true);
        record_launch(&mut presets, "EUC-CN", "ja_JP", false);
        assert_eq!(presets.len(), 1);
        assert!(!presets[0].ok);
        assert_eq!(
            preset_status(&presets, &factory, "euc_cn", "JA-jp"),
            PresetStatus::Failed
        );
        record_launch(&mut presets, "euc-cn", "ja_JP", true);
        assert_eq!(
            preset_status(&presets, &factory, "euc-cn", "ja_JP"),
            PresetStatus::Verified
        );
        assert_eq!(
            preset_status(&presets, &factory, "UTF-8", "zh_CN"),
            PresetStatus::Verified
        );
    }
}
