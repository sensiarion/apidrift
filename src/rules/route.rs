use crate::rules::{Rule, RuleCategory};
use crate::ChangeLevel;
use oas3::spec::ObjectOrReference::Object;
use oas3::spec::Operation;

/// Trait for route-level detection rules
pub trait RouteRule: Rule {
    fn detect(
        path: &str,
        method: &str,
        base: Option<&Operation>,
        current: Option<&Operation>,
    ) -> Vec<Self>
    where
        Self: Sized;
}

// ============================================================================
// ROUTE-LEVEL RULES
// ============================================================================

/// Rule: Entire route was added
#[derive(Debug, Clone)]
pub struct RouteAddedRule {
    pub path: String,
    pub method: String,
}

impl Rule for RouteAddedRule {
    fn name(&self) -> &str {
        "RouteAdded"
    }

    fn description(&self) -> String {
        format!("Route Added: {} {}", self.method.to_uppercase(), self.path)
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Change
    }

    fn context(&self) -> crate::rules::ChangeAnchor {
        crate::rules::ChangeAnchor::Route
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Endpoint
    }
}

impl RouteRule for RouteAddedRule {
    fn detect(
        path: &str,
        method: &str,
        base: Option<&Operation>,
        current: Option<&Operation>,
    ) -> Vec<Self> {
        if base.is_none() && current.is_some() {
            vec![Self {
                path: path.to_string(),
                method: method.to_string(),
            }]
        } else {
            vec![]
        }
    }
}

/// Rule: Entire route was removed
#[derive(Debug, Clone)]
pub struct RouteRemovedRule {
    pub path: String,
    pub method: String,
}

impl Rule for RouteRemovedRule {
    fn name(&self) -> &str {
        "RouteRemoved"
    }

    fn description(&self) -> String {
        format!(
            "Route Removed: {} {}",
            self.method.to_uppercase(),
            self.path
        )
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Breaking
    }

    fn context(&self) -> crate::rules::ChangeAnchor {
        crate::rules::ChangeAnchor::Route
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Endpoint
    }
}

impl RouteRule for RouteRemovedRule {
    fn detect(
        path: &str,
        method: &str,
        base: Option<&Operation>,
        current: Option<&Operation>,
    ) -> Vec<Self> {
        if base.is_some() && current.is_none() {
            vec![Self {
                path: path.to_string(),
                method: method.to_string(),
            }]
        } else {
            vec![]
        }
    }
}

/// Rule: Route description changed
#[derive(Debug, Clone)]
pub struct RouteDescriptionChangedRule {
    pub path: String,
    pub method: String,
    pub old_description: String,
    pub new_description: String,
}

impl Rule for RouteDescriptionChangedRule {
    fn name(&self) -> &str {
        "RouteDescriptionChanged"
    }

    fn description(&self) -> String {
        format!(
            "Description Changed: {} {}",
            self.method.to_uppercase(),
            self.path
        )
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Change
    }

    fn context(&self) -> crate::rules::ChangeAnchor {
        crate::rules::ChangeAnchor::Route
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Endpoint
    }
}

impl RouteRule for RouteDescriptionChangedRule {
    fn detect(
        path: &str,
        method: &str,
        base: Option<&Operation>,
        current: Option<&Operation>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_op), Some(current_op)) => {
                let base_desc = base_op.description.as_deref().unwrap_or("");
                let current_desc = current_op.description.as_deref().unwrap_or("");

                if base_desc != current_desc && !base_desc.is_empty() && !current_desc.is_empty() {
                    vec![Self {
                        path: path.to_string(),
                        method: method.to_string(),
                        old_description: base_desc.to_string(),
                        new_description: current_desc.to_string(),
                    }]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}

/// Rule: Route summary changed
#[derive(Debug, Clone)]
pub struct RouteSummaryChangedRule {
    pub path: String,
    pub method: String,
    pub old_summary: String,
    pub new_summary: String,
}

impl Rule for RouteSummaryChangedRule {
    fn name(&self) -> &str {
        "RouteSummaryChanged"
    }

    fn description(&self) -> String {
        format!(
            "Summary Changed: {} {}",
            self.method.to_uppercase(),
            self.path
        )
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Change
    }

    fn context(&self) -> crate::rules::ChangeAnchor {
        crate::rules::ChangeAnchor::Route
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Endpoint
    }
}

impl RouteRule for RouteSummaryChangedRule {
    fn detect(
        path: &str,
        method: &str,
        base: Option<&Operation>,
        current: Option<&Operation>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_op), Some(current_op)) => {
                let base_summary = base_op.summary.as_deref().unwrap_or("");
                let current_summary = current_op.summary.as_deref().unwrap_or("");

                if base_summary != current_summary
                    && !base_summary.is_empty()
                    && !current_summary.is_empty()
                {
                    vec![Self {
                        path: path.to_string(),
                        method: method.to_string(),
                        old_summary: base_summary.to_string(),
                        new_summary: current_summary.to_string(),
                    }]
                } else {
                    vec![]
                }
            }
            _ => vec![],
        }
    }
}

/// Rule: Required parameter added
#[derive(Debug, Clone)]
pub struct RequiredParameterAddedRule {
    pub path: String,
    pub method: String,
    pub parameter_name: String,
    pub parameter_in: String,
}

impl Rule for RequiredParameterAddedRule {
    fn name(&self) -> &str {
        "RequiredParameterAdded"
    }

    fn description(&self) -> String {
        format!(
            "Required Parameter Added: {} (in: {})",
            self.parameter_name, self.parameter_in
        )
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Breaking
    }

    fn context(&self) -> crate::rules::ChangeAnchor {
        crate::rules::ChangeAnchor::Parameter(self.parameter_name.clone())
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Parameter
    }
}

impl RouteRule for RequiredParameterAddedRule {
    fn detect(
        path: &str,
        method: &str,
        base: Option<&Operation>,
        current: Option<&Operation>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_op), Some(current_op)) => {
                let mut rules = Vec::new();

                // Get parameter (name, location) tuples from base
                // Use string representation of location since ParameterIn doesn't implement Hash/Eq
                let base_params: std::collections::HashSet<_> = base_op
                    .parameters
                    .iter()
                    .filter_map(|p| match p {
                        oas3::spec::ObjectOrReference::Object(param) => {
                            Some((param.name.as_str(), format!("{:?}", param.location)))
                        }
                        _ => None,
                    })
                    .collect();

                // Check current parameters
                for param_ref in &current_op.parameters {
                    if let oas3::spec::ObjectOrReference::Object(param) = param_ref {
                        let param_key = (param.name.as_str(), format!("{:?}", param.location));
                        if param.required.unwrap_or(false) && !base_params.contains(&param_key) {
                            rules.push(Self {
                                path: path.to_string(),
                                method: method.to_string(),
                                parameter_name: param.name.clone(),
                                parameter_in: format!("{:?}", param.location),
                            });
                        }
                    }
                }

                rules
            }
            _ => vec![],
        }
    }
}

/// Rule: Parameter removed
#[derive(Debug, Clone)]
pub struct ParameterRemovedRule {
    pub path: String,
    pub method: String,
    pub parameter_name: String,
    pub parameter_in: String,
}

impl Rule for ParameterRemovedRule {
    fn name(&self) -> &str {
        "ParameterRemoved"
    }

    fn description(&self) -> String {
        format!(
            "Parameter Removed: {} (in: {})",
            self.parameter_name, self.parameter_in
        )
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Breaking
    }

    fn context(&self) -> crate::rules::ChangeAnchor {
        crate::rules::ChangeAnchor::Parameter(self.parameter_name.clone())
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Parameter
    }
}

impl RouteRule for ParameterRemovedRule {
    fn detect(
        path: &str,
        method: &str,
        base: Option<&Operation>,
        current: Option<&Operation>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_op), Some(current_op)) => {
                let mut rules = Vec::new();

                // Get parameter (name, location) tuples from current
                // Use string representation of location since ParameterIn doesn't implement Hash/Eq
                let current_params: std::collections::HashSet<_> = current_op
                    .parameters
                    .iter()
                    .filter_map(|p| match p {
                        Object(param) => {
                            Some((param.name.as_str(), format!("{:?}", param.location)))
                        }
                        _ => None,
                    })
                    .collect();

                // Check base parameters
                for param_ref in &base_op.parameters {
                    if let Object(param) = param_ref {
                        let param_key = (param.name.as_str(), format!("{:?}", param.location));
                        if !current_params.contains(&param_key) {
                            rules.push(Self {
                                path: path.to_string(),
                                method: method.to_string(),
                                parameter_name: param.name.clone(),
                                parameter_in: format!("{:?}", param.location),
                            });
                        }
                    }
                }

                rules
            }
            _ => vec![],
        }
    }
}

/// Rule: Response status code added
#[derive(Debug, Clone)]
pub struct ResponseStatusAddedRule {
    pub path: String,
    pub method: String,
    pub status_code: String,
}

impl Rule for ResponseStatusAddedRule {
    fn name(&self) -> &str {
        "ResponseStatusAdded"
    }

    fn description(&self) -> String {
        format!("Response Status Added: {}", self.status_code)
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Change
    }

    fn context(&self) -> crate::rules::ChangeAnchor {
        crate::rules::ChangeAnchor::ResponseStatus(self.status_code.clone())
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Response
    }
}

impl RouteRule for ResponseStatusAddedRule {
    fn detect(
        path: &str,
        method: &str,
        base: Option<&Operation>,
        current: Option<&Operation>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_op), Some(current_op)) => {
                let mut rules = Vec::new();

                if let Some(current_responses) = &current_op.responses {
                    if let Some(base_responses) = &base_op.responses {
                        for (status_code, _) in current_responses {
                            if !base_responses.contains_key(status_code) {
                                rules.push(Self {
                                    path: path.to_string(),
                                    method: method.to_string(),
                                    status_code: status_code.clone(),
                                });
                            }
                        }
                    }
                }

                rules
            }
            _ => vec![],
        }
    }
}

/// Rule: Response status code removed
#[derive(Debug, Clone)]
pub struct ResponseStatusRemovedRule {
    pub path: String,
    pub method: String,
    pub status_code: String,
}

impl Rule for ResponseStatusRemovedRule {
    fn name(&self) -> &str {
        "ResponseStatusRemoved"
    }

    fn description(&self) -> String {
        format!("Response Status Removed: {}", self.status_code)
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Warning
    }

    fn context(&self) -> crate::rules::ChangeAnchor {
        crate::rules::ChangeAnchor::ResponseStatus(self.status_code.clone())
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Response
    }
}

impl RouteRule for ResponseStatusRemovedRule {
    fn detect(
        path: &str,
        method: &str,
        base: Option<&Operation>,
        current: Option<&Operation>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_op), Some(current_op)) => {
                let mut rules = Vec::new();

                if let Some(base_responses) = &base_op.responses {
                    if let Some(current_responses) = &current_op.responses {
                        for (status_code, _) in base_responses {
                            if !current_responses.contains_key(status_code) {
                                rules.push(Self {
                                    path: path.to_string(),
                                    method: method.to_string(),
                                    status_code: status_code.clone(),
                                });
                            }
                        }
                    }
                }

                rules
            }
            _ => vec![],
        }
    }
}

/// Rule: Request schema changed
#[derive(Debug, Clone)]
pub struct RequestSchemaChangedRule {
    pub path: String,
    pub method: String,
    pub schema_name: String,
    pub content_type: String,
}

impl Rule for RequestSchemaChangedRule {
    fn name(&self) -> &str {
        "RequestSchemaChanged"
    }

    fn description(&self) -> String {
        format!("Request schema '{}' changed", self.schema_name)
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Breaking
    }

    fn context(&self) -> crate::rules::ChangeAnchor {
        crate::rules::ChangeAnchor::Route
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::RequestBody
    }
}

impl RouteRule for RequestSchemaChangedRule {
    fn detect(
        path: &str,
        method: &str,
        base: Option<&Operation>,
        current: Option<&Operation>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_op), Some(current_op)) => {
                let mut rules = Vec::new();

                // Compare request body schemas
                let base_schemas = Self::extract_request_schemas(base_op);
                let current_schemas = Self::extract_request_schemas(current_op);

                // Check for changed schemas
                for (content_type, schema_name) in &current_schemas {
                    if let Some(base_schema_name) = base_schemas.get(content_type) {
                        if base_schema_name != schema_name {
                            rules.push(Self {
                                path: path.to_string(),
                                method: method.to_string(),
                                schema_name: schema_name.clone(),
                                content_type: content_type.clone(),
                            });
                        }
                    }
                }

                rules
            }
            _ => vec![],
        }
    }
}

impl RequestSchemaChangedRule {
    fn extract_request_schemas(op: &Operation) -> std::collections::HashMap<String, String> {
        let mut schemas = std::collections::HashMap::new();

        if let Some(request_body) = &op.request_body {
            if let oas3::spec::ObjectOrReference::Object(body) = request_body {
                for (content_type, media_type) in &body.content {
                    if let Some(schema) = &media_type.schema {
                        if let Some(schema_name) = Self::extract_schema_name(schema) {
                            schemas.insert(content_type.clone(), schema_name);
                        }
                    }
                }
            }
        }

        schemas
    }

    fn extract_schema_name(
        schema: &oas3::spec::ObjectOrReference<oas3::spec::ObjectSchema>,
    ) -> Option<String> {
        match schema {
            oas3::spec::ObjectOrReference::Ref { ref_path, .. } => ref_path
                .strip_prefix("#/components/schemas/")
                .map(|s| s.to_string()),
            _ => None,
        }
    }
}

/// Rule: Response schema changed
#[derive(Debug, Clone)]
pub struct ResponseSchemaChangedRule {
    pub path: String,
    pub method: String,
    pub schema_name: String,
    pub content_type: String,
    pub status_code: String,
}

impl Rule for ResponseSchemaChangedRule {
    fn name(&self) -> &str {
        "ResponseSchemaChanged"
    }

    fn description(&self) -> String {
        format!(
            "Response schema '{}' changed for status {}",
            self.schema_name, self.status_code
        )
    }

    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Breaking
    }

    fn context(&self) -> crate::rules::ChangeAnchor {
        crate::rules::ChangeAnchor::ResponseStatus(self.status_code.clone())
    }

    fn category(&self) -> RuleCategory {
        RuleCategory::Response
    }
}

impl RouteRule for ResponseSchemaChangedRule {
    fn detect(
        path: &str,
        method: &str,
        base: Option<&Operation>,
        current: Option<&Operation>,
    ) -> Vec<Self> {
        match (base, current) {
            (Some(base_op), Some(current_op)) => {
                let mut rules = Vec::new();

                // Compare response schemas
                let base_schemas = Self::extract_response_schemas(base_op);
                let current_schemas = Self::extract_response_schemas(current_op);

                // Check for changed schemas
                for ((status_code, content_type), schema_name) in &current_schemas {
                    if let Some(base_schema_name) =
                        base_schemas.get(&(status_code.clone(), content_type.clone()))
                    {
                        if base_schema_name != schema_name {
                            rules.push(Self {
                                path: path.to_string(),
                                method: method.to_string(),
                                schema_name: schema_name.clone(),
                                content_type: content_type.clone(),
                                status_code: status_code.clone(),
                            });
                        }
                    }
                }

                rules
            }
            _ => vec![],
        }
    }
}

impl ResponseSchemaChangedRule {
    fn extract_response_schemas(
        op: &Operation,
    ) -> std::collections::HashMap<(String, String), String> {
        let mut schemas = std::collections::HashMap::new();

        if let Some(responses) = &op.responses {
            for (status_code, response_ref) in responses {
                if let oas3::spec::ObjectOrReference::Object(response) = response_ref {
                    for (content_type, media_type) in &response.content {
                        if let Some(schema) = &media_type.schema {
                            if let Some(schema_name) = Self::extract_schema_name(schema) {
                                schemas.insert(
                                    (status_code.clone(), content_type.clone()),
                                    schema_name,
                                );
                            }
                        }
                    }
                }
            }
        }

        schemas
    }

    fn extract_schema_name(
        schema: &oas3::spec::ObjectOrReference<oas3::spec::ObjectSchema>,
    ) -> Option<String> {
        match schema {
            oas3::spec::ObjectOrReference::Ref { ref_path, .. } => ref_path
                .strip_prefix("#/components/schemas/")
                .map(|s| s.to_string()),
            _ => None,
        }
    }
}
