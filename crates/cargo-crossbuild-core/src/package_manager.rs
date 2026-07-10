/// Describes package-manager level operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageManagerPlan {
    pub command: String,
}

impl PackageManagerPlan {
    pub fn new(command: impl Into<String>) -> Self {
        Self {
            command: command.into(),
        }
    }

    pub fn command_name(&self) -> &str {
        &self.command
    }
}
