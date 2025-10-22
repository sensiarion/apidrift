use crate::rules::route::*;
use crate::rules::schema::*;
use crate::rules::{MatchResult, RuleViolation};
use log::info;
use oas3::spec::{ObjectOrReference, ObjectSchema, Operation, PathItem, Spec};
use std::collections::{BTreeMap, HashSet};

/// Schema matcher for comparing OpenAPI schemas between versions
pub struct SchemaMatcher<'a> {
    base_schemas: &'a BTreeMap<String, ObjectOrReference<ObjectSchema>>,
    current_schemas: &'a BTreeMap<String, ObjectOrReference<ObjectSchema>>,
    base_spec: &'a Spec,
    current_spec: &'a Spec,
}

impl<'a> SchemaMatcher<'a> {
    pub fn new(
        base_schemas: &'a BTreeMap<String, ObjectOrReference<ObjectSchema>>,
        current_schemas: &'a BTreeMap<String, ObjectOrReference<ObjectSchema>>,
        base_spec: &'a Spec,
        current_spec: &'a Spec,
    ) -> Self {
        Self {
            base_schemas,
            current_schemas,
            base_spec,
            current_spec,
        }
    }

    /// Match schemas between base and current versions
    pub fn match_schemas(&self) -> Vec<MatchResult> {
        let mut results = Vec::new();

        // Find all schema names from both versions
        let mut all_schema_names: std::collections::HashSet<String> =
            self.base_schemas.keys().cloned().collect();
        all_schema_names.extend(self.current_schemas.keys().cloned());

        for schema_name in all_schema_names {
            let base_schema = self.base_schemas.get(&schema_name);
            let current_schema = self.current_schemas.get(&schema_name);

            let violations = self.compare_schemas(&schema_name, base_schema, current_schema);

            if !violations.is_empty() {
                results.push(MatchResult::new(schema_name, violations));
            }
        }

        results
    }

    /// Build full schema info for all current schemas with changes marked
    pub fn build_full_schema_infos(
        &self,
        results: &[MatchResult],
    ) -> Vec<crate::rules::FullSchemaInfo> {
        let mut full_schemas = Vec::new();

        // Process each changed schema
        for result in results {
            if let Some(current_schema_ref) = self.current_schemas.get(&result.name) {
                if let Some(current_schema) =
                    self.resolve_schema_ref(current_schema_ref, self.current_spec)
                {
                    let full_schema = self.build_full_schema_info(
                        &result.name,
                        current_schema,
                        &result.violations,
                    );
                    full_schemas.push(full_schema);
                }
            }
        }

        full_schemas
    }

    /// Build full schema info for a single schema
    fn build_full_schema_info(
        &self,
        schema_name: &str,
        schema: &ObjectSchema,
        violations: &[RuleViolation],
    ) -> crate::rules::FullSchemaInfo {
        use crate::rules::{ChangeAnchor, FullSchemaInfo, SchemaProperty, ViolationInfo};
        use crate::ChangeLevel;

        // Group violations by anchor
        let mut schema_level_violations = Vec::new();
        let mut property_violations: std::collections::HashMap<String, Vec<ViolationInfo>> =
            std::collections::HashMap::new();

        for violation in violations {
            let anchor = violation.context();
            let violation_info = ViolationInfo {
                rule_name: violation.name().to_string(),
                description: violation.description(),
                change_level: match violation.change_level() {
                    ChangeLevel::Breaking => "Breaking".to_string(),
                    ChangeLevel::Warning => "Warning".to_string(),
                    ChangeLevel::Change => "Change".to_string(),
                },
                anchor: format!("{:?}", anchor),
            };

            match anchor {
                ChangeAnchor::Schema | ChangeAnchor::Required => {
                    schema_level_violations.push(violation_info);
                }
                _ => {
                    if let Some(prop_path) = anchor.property_path() {
                        property_violations
                            .entry(prop_path.to_string())
                            .or_insert_with(Vec::new)
                            .push(violation_info);
                    }
                }
            }
        }

        // Build properties list from current schema
        let required_set: HashSet<_> = schema.required.iter().cloned().collect();
        let mut properties = Vec::new();

        for (prop_name, prop_ref) in &schema.properties {
            let prop_schema = self.resolve_schema_ref(prop_ref, self.current_spec);
            let property_type =
                prop_schema.and_then(|s| s.schema_type.as_ref().map(|t| format!("{:?}", t)));
            let format = prop_schema.and_then(|s| s.format.clone());
            let description = prop_schema.and_then(|s| s.description.clone());
            let nullable = prop_schema
                .map(|s| s.is_nullable().unwrap_or(false))
                .unwrap_or(false);
            let enum_values = prop_schema
                .map(|s| s.enum_values.clone())
                .unwrap_or_default();

            let prop_violations = property_violations
                .get(prop_name)
                .cloned()
                .unwrap_or_default();

            properties.push(SchemaProperty {
                name: prop_name.clone(),
                property_type,
                format,
                description,
                required: required_set.contains(prop_name),
                nullable,
                enum_values,
                violations: prop_violations,
            });
        }

        // Sort properties: required first, then by name
        properties.sort_by(|a, b| match (a.required, b.required) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.name.cmp(&b.name),
        });

        // Calculate overall change level
        let change_level = crate::rules::calculate_overall_change_level(violations);
        let (change_level_str, change_level_class) = match change_level {
            ChangeLevel::Breaking => ("Breaking".to_string(), "breaking".to_string()),
            ChangeLevel::Warning => ("Warning".to_string(), "warning".to_string()),
            ChangeLevel::Change => ("Change".to_string(), "change".to_string()),
        };

        FullSchemaInfo {
            name: schema_name.to_string(),
            description: schema.description.clone(),
            properties,
            schema_level_violations,
            change_level: change_level_str,
            change_level_class,
        }
    }

    /// Compare two schemas and find differences
    fn compare_schemas(
        &self,
        schema_name: &str,
        base: Option<&ObjectOrReference<ObjectSchema>>,
        current: Option<&ObjectOrReference<ObjectSchema>>,
    ) -> Vec<RuleViolation> {
        let mut violations = Vec::new();

        // Resolve references first
        let base_schema = base.and_then(|b| self.resolve_schema_ref(b, self.base_spec));
        let current_schema = current.and_then(|c| self.resolve_schema_ref(c, self.current_spec));

        if base.is_some() && current.is_some() && base == current {
            return violations;
        }

        // Use SchemaRule trait for schema-level detection
        violations.extend(self.detect_schema_rule_violations::<SchemaAddedRule>(
            schema_name,
            "",
            base_schema,
            current_schema,
        ));

        violations.extend(self.detect_schema_rule_violations::<SchemaRemovedRule>(
            schema_name,
            "",
            base_schema,
            current_schema,
        ));

        // If both schemas exist, compare their details
        if base.is_some() && current.is_some() {
            if let (Some(base_ref), Some(current_ref)) = (base, current) {
                violations.extend(self.compare_schema_details(
                    schema_name,
                    "",
                    base_ref,
                    current_ref,
                ));
            }
        }

        violations
    }

    /// Resolve a reference to an actual schema
    fn resolve_schema_ref<'b>(
        &self,
        schema_ref: &'b ObjectOrReference<ObjectSchema>,
        spec: &'b Spec,
    ) -> Option<&'b ObjectSchema> {
        match schema_ref {
            ObjectOrReference::Object(obj) => Some(obj),
            ObjectOrReference::Ref { ref_path, .. } => {
                // Parse the reference path (e.g., "#/components/schemas/User")
                if let Some(schema_name) = ref_path.strip_prefix("#/components/schemas/") {
                    // Look up the schema in the spec's components
                    spec.components
                        .as_ref()
                        .and_then(|components| components.schemas.get(schema_name))
                        .and_then(|schema| match schema {
                            ObjectOrReference::Object(obj) => Some(obj),
                            // Handle nested references (though this is rare)
                            ObjectOrReference::Ref { .. } => None,
                        })
                } else {
                    None
                }
            }
        }
    }

    /// Detect all schema-level rule violations using the SchemaRule trait
    fn detect_schema_rule_violations<T: crate::rules::schema::SchemaRule + 'static>(
        &self,
        schema_name: &str,
        property_path: &str,
        base: Option<&ObjectSchema>,
        current: Option<&ObjectSchema>,
    ) -> Vec<RuleViolation> {
        T::detect(schema_name, property_path, base, current)
            .into_iter()
            .map(|rule| RuleViolation::new(Box::new(rule)))
            .collect()
    }

    /// Compare detailed schema properties with depth limit to prevent stack overflow
    fn compare_schema_details(
        &self,
        schema_name: &str,
        property_path: &str,
        base: &ObjectOrReference<ObjectSchema>,
        current: &ObjectOrReference<ObjectSchema>,
    ) -> Vec<RuleViolation> {
        self.compare_schema_details_with_depth(schema_name, property_path, base, current, 0)
    }

    /// Compare detailed schema properties with recursion depth tracking
    fn compare_schema_details_with_depth(
        &self,
        schema_name: &str,
        property_path: &str,
        base: &ObjectOrReference<ObjectSchema>,
        current: &ObjectOrReference<ObjectSchema>,
        depth: usize,
    ) -> Vec<RuleViolation> {
        const MAX_DEPTH: usize = 30; // Prevent infinite recursion
        let mut violations = Vec::new();

        // Stop recursion if we're too deep
        if depth >= MAX_DEPTH {
            return violations;
        }

        // Resolve references to actual schemas
        let base_schema = match self.resolve_schema_ref(base, self.base_spec) {
            Some(schema) => schema,
            None => return violations, // Skip if we can't resolve the reference
        };

        let current_schema = match self.resolve_schema_ref(current, self.current_spec) {
            Some(schema) => schema,
            None => return violations, // Skip if we can't resolve the reference
        };

        // Use SchemaRule trait for detection
        violations.extend(self.detect_schema_rule_violations::<TypeChangedRule>(
            schema_name,
            property_path,
            Some(base_schema),
            Some(current_schema),
        ));

        violations.extend(
            self.detect_schema_rule_violations::<RequiredPropertyAddedRule>(
                schema_name,
                property_path,
                Some(base_schema),
                Some(current_schema),
            ),
        );

        // Use SchemaRule trait for property-level detection
        violations.extend(self.detect_schema_rule_violations::<PropertyAddedRule>(
            schema_name,
            property_path,
            Some(base_schema),
            Some(current_schema),
        ));

        violations.extend(self.detect_schema_rule_violations::<PropertyRemovedRule>(
            schema_name,
            property_path,
            Some(base_schema),
            Some(current_schema),
        ));

        // Detect properties that were removed from required array but still exist as optional
        let base_required: std::collections::HashSet<_> = base_schema.required.iter().collect();
        let current_required: std::collections::HashSet<_> =
            current_schema.required.iter().collect();
        let current_props_keys: std::collections::HashSet<_> =
            current_schema.properties.keys().collect();

        for prop in base_required.difference(&current_required) {
            // Only if the property still exists (made optional rather than removed)
            if current_props_keys.contains(prop) {
                violations.push(RuleViolation::new(Box::new(PropertyRemovedRule {
                    schema_name: schema_name.to_string(),
                    property_path: property_path.to_string(),
                    property_name: (*prop).clone(),
                    was_required: true,
                    totally_removed: false, // Property still exists, just made optional
                })));
            }
        }

        // Recursively compare nested properties
        let base_props = &base_schema.properties;
        let current_props = &current_schema.properties;

        for (prop_name, current_prop) in current_props {
            if let Some(base_prop) = base_props.get(prop_name) {
                // Build nested property path
                let nested_path = if property_path.is_empty() {
                    prop_name.clone()
                } else {
                    format!("{}.{}", property_path, prop_name)
                };
                let prop_violations = self.compare_schema_details_with_depth(
                    schema_name,
                    &nested_path,
                    base_prop,
                    current_prop,
                    depth + 1,
                );
                violations.extend(prop_violations);
            }
        }

        // Use SchemaRule trait for all other detections
        violations.extend(
            self.detect_schema_rule_violations::<DescriptionChangedRule>(
                schema_name,
                property_path,
                Some(base_schema),
                Some(current_schema),
            ),
        );

        violations.extend(self.detect_schema_rule_violations::<EnumValuesAddedRule>(
            schema_name,
            property_path,
            Some(base_schema),
            Some(current_schema),
        ));

        violations.extend(self.detect_schema_rule_violations::<EnumValuesRemovedRule>(
            schema_name,
            property_path,
            Some(base_schema),
            Some(current_schema),
        ));

        violations.extend(self.detect_schema_rule_violations::<FormatChangedRule>(
            schema_name,
            property_path,
            Some(base_schema),
            Some(current_schema),
        ));

        violations.extend(self.detect_schema_rule_violations::<NullableChangedRule>(
            schema_name,
            property_path,
            Some(base_schema),
            Some(current_schema),
        ));

        // Compare array items (for now, skip Schema enum handling)
        // TODO: Implement proper array items comparison

        violations
    }
}

/// Route matcher for comparing OpenAPI routes/paths between versions
pub struct RouteMatcher<'a> {
    base_spec: &'a Spec,
    current_spec: &'a Spec,
}

/// Represents route information with associated schemas
#[derive(Debug, Clone)]
pub struct RouteInfo {
    pub path: String,
    pub method: String,
    pub request_schemas: Vec<SchemaReference>,
    pub response_schemas: Vec<SchemaReference>,
}

/// Reference to a schema used in a route
#[derive(Debug, Clone)]
pub struct SchemaReference {
    pub schema_name: String,
    pub content_type: String,
    pub location: SchemaLocation,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SchemaLocation {
    RequestBody,
    Response(String), // Status code
}

impl<'a> RouteMatcher<'a> {
    pub fn new(base_spec: &'a Spec, current_spec: &'a Spec) -> Self {
        Self {
            base_spec,
            current_spec,
        }
    }

    /// Match routes between base and current versions
    pub fn match_routes(&self) -> Vec<MatchResult> {
        self.match_routes_with_schema_violations(&[])
    }

    /// Match routes between base and current versions, including schema violations
    pub fn match_routes_with_schema_violations(
        &self,
        schema_results: &[MatchResult],
    ) -> Vec<MatchResult> {
        let mut results = Vec::new();

        // Get all paths from both versions
        let base_paths = self.base_spec.paths.as_ref();
        let current_paths = self.current_spec.paths.as_ref();

        // Collect all unique path keys
        let mut all_paths: HashSet<String> = HashSet::new();
        if let Some(paths) = base_paths {
            all_paths.extend(paths.keys().cloned());
        }
        if let Some(paths) = current_paths {
            all_paths.extend(paths.keys().cloned());
        }

        for path in all_paths {
            let base_path_item = base_paths.and_then(|p| p.get(&path));
            let current_path_item = current_paths.and_then(|p| p.get(&path));

            // Compare each HTTP method
            let methods = vec!["get", "post", "put", "delete", "patch", "head", "options"];

            for method in methods {
                let base_op = base_path_item.and_then(|p| self.get_operation(p, method));
                let current_op = current_path_item.and_then(|p| self.get_operation(p, method));

                // Skip if both are None
                if base_op.is_none() && current_op.is_none() {
                    continue;
                }

                // Check if operations are identical (but still process if schemas have changes)
                let operations_identical = if base_op.is_some() && current_op.is_some() {
                    base_op == current_op
                } else {
                    false
                };

                let mut violations = Vec::new();

                // Only compare operations if they're not identical
                if !operations_identical {
                    info!("Route {} {} not same ", method, path);
                    violations.extend(self.compare_operations(&path, method, base_op, current_op));
                }

                // Always check for schema violations for this route's schemas
                if let Some(current_op) = current_op {
                    let route_schemas = self.extract_route_schemas(&path, method, current_op);
                    violations.extend(
                        self.get_schema_violations_for_route(&route_schemas, schema_results),
                    );
                }

                if !violations.is_empty() {
                    let route_name = format!("{} {}", method.to_uppercase(), path);
                    results.push(MatchResult::new(route_name, violations));
                }
            }
        }

        results
    }

    /// Get operation for a specific HTTP method from PathItem
    fn get_operation<'b>(&self, path_item: &'b PathItem, method: &str) -> Option<&'b Operation> {
        match method {
            "get" => path_item.get.as_ref(),
            "post" => path_item.post.as_ref(),
            "put" => path_item.put.as_ref(),
            "delete" => path_item.delete.as_ref(),
            "patch" => path_item.patch.as_ref(),
            "head" => path_item.head.as_ref(),
            "options" => path_item.options.as_ref(),
            _ => None,
        }
    }

    /// Compare two operations and detect rule violations
    fn compare_operations(
        &self,
        path: &str,
        method: &str,
        base: Option<&Operation>,
        current: Option<&Operation>,
    ) -> Vec<RuleViolation> {
        let mut violations = Vec::new();

        // Detect route-level changes
        violations.extend(
            self.detect_route_rule_violations::<RouteAddedRule>(path, method, base, current),
        );

        violations.extend(
            self.detect_route_rule_violations::<RouteRemovedRule>(path, method, base, current),
        );

        // If both operations exist, compare details
        if base.is_some() && current.is_some() {
            violations.extend(
                self.detect_route_rule_violations::<RouteDescriptionChangedRule>(
                    path, method, base, current,
                ),
            );

            violations.extend(
                self.detect_route_rule_violations::<RouteSummaryChangedRule>(
                    path, method, base, current,
                ),
            );

            violations.extend(
                self.detect_route_rule_violations::<RequiredParameterAddedRule>(
                    path, method, base, current,
                ),
            );

            violations.extend(
                self.detect_route_rule_violations::<ParameterRemovedRule>(
                    path, method, base, current,
                ),
            );

            violations.extend(
                self.detect_route_rule_violations::<ResponseStatusAddedRule>(
                    path, method, base, current,
                ),
            );

            violations.extend(
                self.detect_route_rule_violations::<ResponseStatusRemovedRule>(
                    path, method, base, current,
                ),
            );

            violations.extend(
                self.detect_route_rule_violations::<RequestSchemaChangedRule>(
                    path, method, base, current,
                ),
            );

            violations.extend(
                self.detect_route_rule_violations::<ResponseSchemaChangedRule>(
                    path, method, base, current,
                ),
            );
        }

        violations
    }

    /// Detect route rule violations using the RouteRule trait
    fn detect_route_rule_violations<T: RouteRule + 'static>(
        &self,
        path: &str,
        method: &str,
        base: Option<&Operation>,
        current: Option<&Operation>,
    ) -> Vec<RuleViolation> {
        T::detect(path, method, base, current)
            .into_iter()
            .map(|rule| RuleViolation::new(Box::new(rule)))
            .collect()
    }

    /// Extract schema references from an operation
    pub fn extract_route_schemas(
        &self,
        path: &str,
        method: &str,
        operation: &Operation,
    ) -> RouteInfo {
        let mut request_schemas = Vec::new();
        let mut response_schemas = Vec::new();

        // Extract request body schemas
        if let Some(request_body) = &operation.request_body {
            if let ObjectOrReference::Object(body) = request_body {
                for (content_type, media_type) in &body.content {
                    if let Some(schema) = &media_type.schema {
                        if let Some(schema_name) = Self::extract_schema_name_static(schema) {
                            request_schemas.push(SchemaReference {
                                schema_name,
                                content_type: content_type.clone(),
                                location: SchemaLocation::RequestBody,
                            });
                        }
                    }
                }
            }
        }

        // Extract response schemas
        if let Some(responses) = &operation.responses {
            for (status_code, response_ref) in responses {
                if let ObjectOrReference::Object(response) = response_ref {
                    for (content_type, media_type) in &response.content {
                        if let Some(schema) = &media_type.schema {
                            if let Some(schema_name) = Self::extract_schema_name_static(schema) {
                                response_schemas.push(SchemaReference {
                                    schema_name,
                                    content_type: content_type.clone(),
                                    location: SchemaLocation::Response(status_code.clone()),
                                });
                            }
                        }
                    }
                }
            }
        }

        RouteInfo {
            path: path.to_string(),
            method: method.to_string(),
            request_schemas,
            response_schemas,
        }
    }

    /// Extract schema name from a schema reference (static method)
    fn extract_schema_name_static(schema: &ObjectOrReference<ObjectSchema>) -> Option<String> {
        match schema {
            ObjectOrReference::Ref { ref_path, .. } => ref_path
                .strip_prefix("#/components/schemas/")
                .map(|s| s.to_string()),
            _ => None,
        }
    }

    /// Get all routes with their schema information for the current spec
    pub fn get_all_routes_with_schemas(&self) -> Vec<RouteInfo> {
        let mut routes = Vec::new();

        if let Some(paths) = &self.current_spec.paths {
            for (path, path_item) in paths {
                let methods = vec!["get", "post", "put", "delete", "patch", "head", "options"];

                for method in methods {
                    if let Some(operation) = self.get_operation(path_item, method) {
                        routes.push(self.extract_route_schemas(path, method, operation));
                    }
                }
            }
        }

        routes
    }

    /// Get schema violations for schemas used in a route
    fn get_schema_violations_for_route(
        &self,
        route_schemas: &RouteInfo,
        schema_results: &[MatchResult],
    ) -> Vec<RuleViolation> {
        let mut violations = Vec::new();

        // Create a map of schema names to their violations for quick lookup
        let schema_violations_map: std::collections::HashMap<String, Vec<&RuleViolation>> =
            schema_results
                .iter()
                .flat_map(|result| {
                    result
                        .violations
                        .iter()
                        .map(|violation| (result.name.clone(), violation))
                })
                .fold(
                    std::collections::HashMap::new(),
                    |mut acc, (schema_name, violation)| {
                        acc.entry(schema_name)
                            .or_insert_with(Vec::new)
                            .push(violation);
                        acc
                    },
                );

        // Check request schemas
        for schema_ref in &route_schemas.request_schemas {
            if let Some(schema_violations) = schema_violations_map.get(&schema_ref.schema_name) {
                for violation in schema_violations {
                    violations.push(RuleViolation::new(Box::new(
                        RequestSchemaViolationWrapper {
                            schema_name: schema_ref.schema_name.clone(),
                            content_type: schema_ref.content_type.clone(),
                            violation: RuleViolation::new(Box::new(SchemaViolationInfo {
                                name: violation.name().to_string(),
                                description: violation.description(),
                                change_level: violation.change_level(),
                                context: violation.context(),
                                category: violation.category(),
                            })),
                        },
                    )));
                }
            }
        }

        // Check response schemas
        for schema_ref in &route_schemas.response_schemas {
            if let Some(schema_violations) = schema_violations_map.get(&schema_ref.schema_name) {
                for violation in schema_violations {
                    violations.push(RuleViolation::new(Box::new(
                        ResponseSchemaViolationWrapper {
                            schema_name: schema_ref.schema_name.clone(),
                            content_type: schema_ref.content_type.clone(),
                            status_code: match &schema_ref.location {
                                SchemaLocation::Response(code) => code.clone(),
                                _ => "unknown".to_string(),
                            },
                            violation: RuleViolation::new(Box::new(SchemaViolationInfo {
                                name: violation.name().to_string(),
                                description: violation.description(),
                                change_level: violation.change_level(),
                                context: violation.context(),
                                category: violation.category(),
                            })),
                        },
                    )));
                }
            }
        }

        violations
    }
}

/// Simple wrapper to store violation info without cloning RuleViolation
#[derive(Debug)]
struct SchemaViolationInfo {
    name: String,
    description: String,
    change_level: crate::ChangeLevel,
    context: crate::rules::ChangeAnchor,
    category: crate::rules::RuleCategory,
}

impl crate::rules::Rule for SchemaViolationInfo {
    fn name(&self) -> &str {
        &self.name
    }

    fn description(&self) -> String {
        self.description.clone()
    }

    fn change_level(&self) -> crate::ChangeLevel {
        self.change_level.clone()
    }

    fn context(&self) -> crate::rules::ChangeAnchor {
        self.context.clone()
    }

    fn category(&self) -> crate::rules::RuleCategory {
        self.category.clone()
    }
}

/// Wrapper to add schema context to schema violations for routes
#[derive(Debug)]
struct RequestSchemaViolationWrapper {
    schema_name: String,
    content_type: String,
    violation: RuleViolation,
}

impl crate::rules::Rule for RequestSchemaViolationWrapper {
    fn name(&self) -> &str {
        "RequestSchemaViolation"
    }

    fn description(&self) -> String {
        format!(
            "Request schema '{}' ({}) - {}",
            self.schema_name,
            self.content_type,
            self.violation.description()
        )
    }

    fn change_level(&self) -> crate::ChangeLevel {
        self.violation.change_level()
    }

    fn context(&self) -> crate::rules::ChangeAnchor {
        crate::rules::ChangeAnchor::Route
    }

    fn category(&self) -> crate::rules::RuleCategory {
        crate::rules::RuleCategory::RequestBody
    }
}

/// Wrapper to add schema context to schema violations for routes
#[derive(Debug)]
struct ResponseSchemaViolationWrapper {
    schema_name: String,
    content_type: String,
    status_code: String,
    violation: RuleViolation,
}

impl crate::rules::Rule for ResponseSchemaViolationWrapper {
    fn name(&self) -> &str {
        "ResponseSchemaViolation"
    }

    fn description(&self) -> String {
        format!(
            "Response schema '{}' ({}) for status {} - {}",
            self.schema_name,
            self.content_type,
            self.status_code,
            self.violation.description()
        )
    }

    fn change_level(&self) -> crate::ChangeLevel {
        self.violation.change_level()
    }

    fn context(&self) -> crate::rules::ChangeAnchor {
        crate::rules::ChangeAnchor::ResponseStatus(self.status_code.clone())
    }

    fn category(&self) -> crate::rules::RuleCategory {
        crate::rules::RuleCategory::Response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rules::Rule;
    use crate::ChangeLevel;

    #[test]
    fn test_change_level_detection() {
        // Breaking changes
        let removed_rule = SchemaRemovedRule {
            schema_name: "Test".to_string(),
        };
        assert!(matches!(removed_rule.change_level(), ChangeLevel::Breaking));

        let type_changed_rule = TypeChangedRule {
            schema_name: "Test".to_string(),
            property_path: "".to_string(),
            old_type: "String".to_string(),
            new_type: "Number".to_string(),
        };
        assert!(matches!(
            type_changed_rule.change_level(),
            ChangeLevel::Breaking
        ));

        // Warnings
        let format_changed_rule = FormatChangedRule {
            schema_name: "Test".to_string(),
            property_path: "".to_string(),
            old_format: Some("email".to_string()),
            new_format: Some("uri".to_string()),
        };
        assert!(matches!(
            format_changed_rule.change_level(),
            ChangeLevel::Warning
        ));

        // Non-breaking changes
        let added_rule = SchemaAddedRule {
            schema_name: "Test".to_string(),
        };
        assert!(matches!(added_rule.change_level(), ChangeLevel::Change));
    }
}
