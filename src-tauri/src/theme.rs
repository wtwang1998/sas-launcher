use serde::{Deserialize, Serialize};

/// Which palette the window uses. `System` keeps following the Windows theme.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
pub enum ThemePreference {
    #[default]
    System,
    Dark,
    Light,
}

impl ThemePreference {
    /// `None` asks the window manager to follow the system theme again.
    pub fn window_theme(self) -> Option<tauri::Theme> {
        match self {
            Self::System => None,
            Self::Dark => Some(tauri::Theme::Dark),
            Self::Light => Some(tauri::Theme::Light),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preference_uses_kebab_case_json() {
        assert_eq!(
            serde_json::to_string(&ThemePreference::Dark).unwrap(),
            "\"dark\""
        );
        assert_eq!(
            serde_json::to_string(&ThemePreference::System).unwrap(),
            "\"system\""
        );
        let parsed: ThemePreference = serde_json::from_str("\"light\"").unwrap();
        assert!(matches!(parsed, ThemePreference::Light));
    }

    #[test]
    fn default_follows_the_system_without_overriding_the_window() {
        assert!(matches!(
            ThemePreference::default(),
            ThemePreference::System
        ));
        assert!(ThemePreference::System.window_theme().is_none());
        assert!(matches!(
            ThemePreference::Dark.window_theme(),
            Some(tauri::Theme::Dark)
        ));
        assert!(matches!(
            ThemePreference::Light.window_theme(),
            Some(tauri::Theme::Light)
        ));
    }
}
