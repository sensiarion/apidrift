pub mod schema;
pub mod route;

use crate::ChangeLevel;

/// Base trait for all comparison rules
/// Rules detect specific types of differences between API versions
pub trait Rule: std::fmt::Debug {
    /// The name of the rule (e.g., "SchemaRemoved", "TypeChanged")
    fn name(&self) -> &str;
    
    /// Human-readable description of what this rule detected
    fn description(&self) -> String;
    
    /// The severity/change level of this rule violation
    fn change_level(&self) -> ChangeLevel;
    
    /// Context where this rule was violated (e.g., "schema: User", "property: email")
    fn context(&self) -> String;
    
    /// Optional: the category of the rule (schema, endpoint, parameter, etc.)
    fn category(&self) -> RuleCategory {
        RuleCategory::Schema
    }
}

/// Category of rule to support different API aspects
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleCategory {
    Schema,
    Endpoint,
    Parameter,
    Response,
    RequestBody,
}

/// Wrapper for dynamic rule violations
#[derive(Debug)]
pub struct RuleViolation {
    rule: Box<dyn Rule>,
}

impl RuleViolation {
    pub fn new(rule: Box<dyn Rule>) -> Self {
        Self { rule }
    }
    
    pub fn rule(&self) -> &dyn Rule {
        self.rule.as_ref()
    }
    
    pub fn name(&self) -> &str {
        self.rule.name()
    }
    
    pub fn description(&self) -> String {
        self.rule.description()
    }
    
    pub fn change_level(&self) -> ChangeLevel {
        self.rule.change_level()
    }
    
    pub fn context(&self) -> String {
        self.rule.context()
    }
    
    pub fn category(&self) -> RuleCategory {
        self.rule.category()
    }
}

/// Result of matching with rule violations
#[derive(Debug)]
pub struct MatchResult {
    pub name: String,
    pub violations: Vec<RuleViolation>,
    pub change_level: ChangeLevel,
}

impl MatchResult {
    pub fn new(name: String, violations: Vec<RuleViolation>) -> Self {
        let change_level = calculate_overall_change_level(&violations);
        Self {
            name,
            violations,
            change_level,
        }
    }
}

/// Calculate the overall change level from a list of violations
pub fn calculate_overall_change_level(violations: &[RuleViolation]) -> ChangeLevel {
    let mut has_breaking = false;
    let mut has_warning = false;

    for violation in violations {
        match violation.change_level() {
            ChangeLevel::Breaking => has_breaking = true,
            ChangeLevel::Warning => has_warning = true,
            ChangeLevel::Change => {}
        }
    }

    if has_breaking {
        ChangeLevel::Breaking
    } else if has_warning {
        ChangeLevel::Warning
    } else {
        ChangeLevel::Change
    }
}

