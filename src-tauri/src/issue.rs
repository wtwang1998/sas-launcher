use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize)]
pub struct Issue {
    pub code: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl Issue {
    pub fn new(code: &'static str) -> Self {
        Self { code, detail: None }
    }

    pub fn with_detail(code: &'static str, detail: impl ToString) -> Self {
        Self {
            code,
            detail: Some(detail.to_string()),
        }
    }
}

/// A persisted copy of an [`Issue`]: `code` is owned so it can be read back from
/// a session manifest. Serializes to the same `{code, detail?}` shape the
/// frontend already understands.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FailureNote {
    pub code: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

impl From<&Issue> for FailureNote {
    fn from(issue: &Issue) -> Self {
        Self {
            code: issue.code.to_string(),
            detail: issue.detail.clone(),
        }
    }
}
