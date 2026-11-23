use serde::{Deserialize, Serialize};
use windows_sys::Win32::Globalization::{GetUserPreferredUILanguages, MUI_LANGUAGE_NAME};

#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LanguagePreference {
    #[default]
    System,
    En,
    #[serde(rename = "zh-Hans")]
    ZhHans,
}

impl LanguagePreference {
    pub fn resolve(self, system: &str) -> &'static str {
        match self {
            Self::ZhHans => "zh-Hans",
            Self::En => "en",
            Self::System if is_simplified_chinese(system) => "zh-Hans",
            Self::System => "en",
        }
    }
}

fn is_simplified_chinese(language: &str) -> bool {
    let language = language.to_ascii_lowercase();
    matches!(language.as_str(), "zh-cn" | "zh-sg" | "zh-hans") || language.starts_with("zh-hans-")
}

pub fn system_language() -> String {
    let mut count = 0;
    let mut size = 0;
    if unsafe {
        GetUserPreferredUILanguages(
            MUI_LANGUAGE_NAME,
            &mut count,
            std::ptr::null_mut(),
            &mut size,
        )
    } != 0
        && size > 1
    {
        let mut buffer = vec![0; size as usize];
        if unsafe {
            GetUserPreferredUILanguages(
                MUI_LANGUAGE_NAME,
                &mut count,
                buffer.as_mut_ptr(),
                &mut size,
            )
        } != 0
        {
            let length = buffer
                .iter()
                .position(|ch| *ch == 0)
                .unwrap_or(buffer.len());
            if length > 0 {
                return String::from_utf16_lossy(&buffer[..length]);
            }
        }
    }
    "en".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn system_language_and_explicit_preferences_are_independent() {
        assert_eq!(LanguagePreference::System.resolve("zh-CN"), "zh-Hans");
        assert_eq!(LanguagePreference::System.resolve("en-GB"), "en");
        assert_eq!(LanguagePreference::System.resolve("de-DE"), "en");
        assert_eq!(LanguagePreference::System.resolve("zh-Hant-TW"), "en");
        assert_eq!(LanguagePreference::ZhHans.resolve("de-DE"), "zh-Hans");
        assert_eq!(LanguagePreference::En.resolve("zh-CN"), "en");
    }
}
