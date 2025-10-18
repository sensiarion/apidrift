pub mod matcher;

/// Level of specific change
#[derive(Debug, Clone, PartialEq)]
pub enum ChangeLevel {
    Breaking,
    Warning,
    Change,
}

