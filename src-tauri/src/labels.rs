//! The handful of strings the backend renders itself: the tray menu and the generated
//! configuration names. They live in `shared/labels.json` so the frontend imports the
//! same template instead of keeping a second, Chinese-only copy.
use serde::Deserialize;
use std::{collections::HashMap, sync::OnceLock};

const RAW: &str = include_str!("../../shared/labels.json");

/// The languages the catalogue is written in; every key must have all of them.
pub const LANGUAGES: [&str; 8] = ["zh-Hans", "en", "zh-Hant", "ja", "ko", "de", "es", "fr"];

#[derive(Deserialize)]
struct Catalog(HashMap<String, HashMap<String, String>>);

fn catalog() -> &'static Catalog {
    static CATALOG: OnceLock<Catalog> = OnceLock::new();
    CATALOG.get_or_init(|| serde_json::from_str(RAW).expect("shared/labels.json is valid"))
}

/// A backend label. An unknown key renders as the key itself, so a missing entry is
/// visible in the UI instead of blank.
pub fn text(language: &str, key: &str) -> String {
    catalog()
        .0
        .get(key)
        .and_then(|entry| entry.get(language).or_else(|| entry.get("en")))
        .cloned()
        .unwrap_or_else(|| key.to_owned())
}

/// `用户配置（衍生自 {base}）#2` with the shared template's placeholders filled in.
pub fn derived_config_name(language: &str, base: &str, number: u32) -> String {
    text(language, "config.derivedName")
        .replace("{base}", base)
        .replace("{index}", &number.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn every_label_is_written_in_every_language() {
        for (key, entry) in &catalog().0 {
            for language in LANGUAGES {
                let value = entry
                    .get(language)
                    .unwrap_or_else(|| panic!("{key} has no {language}"));
                assert!(!value.trim().is_empty(), "{key} in {language}");
            }
        }
    }

    #[test]
    fn a_language_without_a_translation_reads_in_english() {
        assert_eq!(text("th", "tray.quit"), text("en", "tray.quit"));
    }

    #[test]
    fn the_backend_only_names_labels_the_catalogue_has() {
        let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
        let mut unknown: Vec<String> = Vec::new();
        for entry in std::fs::read_dir(manifest.join("src")).unwrap().flatten() {
            let path = entry.path();
            if path.extension().and_then(|value| value.to_str()) != Some("rs") {
                continue;
            }
            let source = std::fs::read_to_string(&path).unwrap();
            for marker in ["\"config.", "\"tray."] {
                let mut rest = source.as_str();
                while let Some(at) = rest.find(marker) {
                    let after = &rest[at + 1..];
                    if let Some(end) = after.find('"') {
                        let key = &after[..end];
                        // The marker literals themselves end at the dot.
                        let named = !key.ends_with('.');
                        if named
                            && !catalog().0.contains_key(key)
                            && !unknown.iter().any(|known| known == key)
                        {
                            unknown.push(key.to_owned());
                        }
                    }
                    rest = &rest[at + marker.len()..];
                }
            }
        }
        assert!(unknown.is_empty(), "shared/labels.json has no {unknown:?}");
    }
}
