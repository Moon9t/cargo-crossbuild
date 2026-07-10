/// Describes a wrapper command and its target invocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WrapperPlan {
    pub wrapper: String,
    pub target: String,
}

impl WrapperPlan {
    pub fn new(wrapper: impl Into<String>, target: impl Into<String>) -> Self {
        Self {
            wrapper: wrapper.into(),
            target: target.into(),
        }
    }

    pub fn invocation(&self) -> String {
        format!("{} {}", self.wrapper, self.target)
    }
}
