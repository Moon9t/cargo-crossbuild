/// Describes release orchestration metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReleasePlan {
    pub version: String,
}

impl ReleasePlan {
    pub fn new(version: impl Into<String>) -> Self {
        Self {
            version: version.into(),
        }
    }

    pub fn is_prerelease(&self) -> bool {
        self.version.contains('-')
    }

    pub fn tag_name(&self) -> String {
        format!("v{}", self.version)
    }
}
