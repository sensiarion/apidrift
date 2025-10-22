use apidrift::ChangeLevel;
use oas3::spec::{
    ObjectOrReference, ObjectSchema, Operation, Parameter, ParameterIn, PathItem, Response,
    SchemaType, SchemaTypeSet, Spec,
};
use std::collections::BTreeMap;

// ============================================================================
// HELPER MACROS AND FUNCTIONS
// ============================================================================

/// Macro to create a parameter with common fields
macro_rules! param {
    ($name:expr, $location:expr, $required:expr) => {
        ObjectOrReference::Object(Parameter {
            name: $name.to_string(),
            location: $location,
            required: Some($required),
            schema: Some(ObjectOrReference::Object(ObjectSchema {
                schema_type: Some(SchemaTypeSet::Single(SchemaType::String)),
                ..Default::default()
            })),
            description: None,
            deprecated: None,
            allow_empty_value: None,
            style: None,
            explode: None,
            allow_reserved: None,
            example: None,
            examples: BTreeMap::new(),
            content: None,
            extensions: BTreeMap::new(),
        })
    };
}

/// Create a basic response
fn response(description: &str) -> ObjectOrReference<Response> {
    ObjectOrReference::Object(Response {
        description: Some(description.to_string()),
        ..Default::default()
    })
}

/// Create an operation with optional fields
fn operation() -> Operation {
    Operation {
        responses: Some(BTreeMap::new()),
        ..Default::default()
    }
}

/// Create a minimal OpenAPI spec for testing
fn create_spec() -> Spec {
    Spec {
        openapi: "3.0.0".to_string(),
        info: oas3::spec::Info {
            title: "Test API".to_string(),
            version: "1.0.0".to_string(),
            summary: None,
            description: None,
            terms_of_service: None,
            contact: None,
            license: None,
            extensions: BTreeMap::new(),
        },
        servers: vec![],
        paths: Some(BTreeMap::new()),
        webhooks: BTreeMap::new(),
        components: None,
        security: vec![],
        tags: vec![],
        external_docs: None,
        extensions: BTreeMap::new(),
    }
}

/// Helper to add a path with operation to a spec
fn add_path(spec: &mut Spec, path: &str, method: &str, op: Operation) {
    let paths = spec.paths.as_mut().unwrap();
    let path_item = paths.entry(path.to_string()).or_insert_with(|| PathItem {
        reference: None,
        summary: None,
        description: None,
        get: None,
        put: None,
        post: None,
        delete: None,
        options: None,
        head: None,
        patch: None,
        trace: None,
        servers: vec![],
        parameters: vec![],
        extensions: BTreeMap::new(),
    });

    match method {
        "get" => path_item.get = Some(op),
        "post" => path_item.post = Some(op),
        "put" => path_item.put = Some(op),
        "delete" => path_item.delete = Some(op),
        "patch" => path_item.patch = Some(op),
        "head" => path_item.head = Some(op),
        "options" => path_item.options = Some(op),
        _ => panic!("Unsupported HTTP method: {}", method),
    }
}

// ============================================================================
// ROUTE-LEVEL RULE TESTS
// ============================================================================

#[cfg(test)]
mod route_tests {
    use super::*;
    use apidrift::matcher::RouteMatcher;
    use apidrift::rules::route::*;

    #[test]
    fn test_route_added_rule_detection() {
        let base = create_spec();
        let mut current = create_spec();

        // Add a new route to current
        let mut op = operation();
        op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));
        add_path(&mut current, "/users", "get", op);

        let matcher = RouteMatcher::new(&base, &current);
        let results = matcher.match_routes();

        let route_result = results.iter().find(|r| r.name == "GET /users");
        assert!(route_result.is_some(), "Should detect new route");

        let route = route_result.unwrap();
        assert_eq!(route.change_level, ChangeLevel::Change);

        let has_route_added = route.violations.iter().any(|v| v.name() == "RouteAdded");
        assert!(has_route_added, "Should have RouteAdded violation");
    }

    #[test]
    fn test_route_added_rule_no_detection() {
        // Test that RouteAddedRule doesn't detect when route exists in both
        let mut base = create_spec();
        let mut current = create_spec();

        let op = operation();
        add_path(&mut base, "/users", "get", op.clone());
        add_path(&mut current, "/users", "get", op);

        let violations =
            RouteAddedRule::detect("/users", "get", base.paths.as_ref().and_then(|p| p.get("/users")).and_then(|pi| pi.get.as_ref()),
                                  current.paths.as_ref().and_then(|p| p.get("/users")).and_then(|pi| pi.get.as_ref()));

        assert_eq!(violations.len(), 0, "Should not detect route as added");
    }

    #[test]
    fn test_route_removed_rule_detection() {
        let mut base = create_spec();
        let current = create_spec();

        // Add route to base but not to current
        let mut op = operation();
        op.responses = Some(BTreeMap::from([("204".to_string(), response("Deleted"))]));
        add_path(&mut base, "/users/{id}", "delete", op);

        let matcher = RouteMatcher::new(&base, &current);
        let results = matcher.match_routes();

        let route_result = results.iter().find(|r| r.name == "DELETE /users/{id}");
        assert!(route_result.is_some(), "Should detect removed route");

        let route = route_result.unwrap();
        assert_eq!(route.change_level, ChangeLevel::Breaking);

        let has_route_removed = route
            .violations
            .iter()
            .any(|v| v.name() == "RouteRemoved");
        assert!(has_route_removed, "Should have RouteRemoved violation");
    }

    #[test]
    fn test_route_removed_rule_no_detection() {
        // Test that RouteRemovedRule doesn't fire when route exists in both
        let mut base = create_spec();
        let mut current = create_spec();

        let op = operation();
        add_path(&mut base, "/users", "get", op.clone());
        add_path(&mut current, "/users", "get", op);

        let violations = RouteRemovedRule::detect(
            "/users",
            "get",
            base.paths.as_ref().and_then(|p| p.get("/users")).and_then(|pi| pi.get.as_ref()),
            current.paths.as_ref().and_then(|p| p.get("/users")).and_then(|pi| pi.get.as_ref()),
        );

        assert_eq!(violations.len(), 0, "Should not detect route as removed");
    }

    #[test]
    fn test_route_summary_changed_rule_detection() {
        let mut base = create_spec();
        let mut current = create_spec();

        let mut base_op = operation();
        base_op.summary = Some("Get users".to_string());
        base_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        let mut current_op = operation();
        current_op.summary = Some("Get all users".to_string());
        current_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        add_path(&mut base, "/users", "get", base_op);
        add_path(&mut current, "/users", "get", current_op);

        let matcher = RouteMatcher::new(&base, &current);
        let results = matcher.match_routes();

        let route_result = results.iter().find(|r| r.name == "GET /users");
        assert!(route_result.is_some());

        let route = route_result.unwrap();
        let has_summary_changed = route
            .violations
            .iter()
            .any(|v| v.name() == "RouteSummaryChanged");
        assert!(
            has_summary_changed,
            "Should detect summary change in GET /users"
        );
        assert_eq!(route.change_level, ChangeLevel::Change);
    }

    #[test]
    fn test_route_summary_changed_rule_no_detection() {
        // Test that empty summaries don't trigger detection
        let mut base_op = operation();
        base_op.summary = Some("Same".to_string());

        let mut current_op = operation();
        current_op.summary = Some("Same".to_string());

        let violations = RouteSummaryChangedRule::detect(
            "/users",
            "get",
            Some(&base_op),
            Some(&current_op),
        );

        assert_eq!(
            violations.len(),
            0,
            "Should not detect change when summaries are identical"
        );
    }

    #[test]
    fn test_route_description_changed_rule_detection() {
        let mut base = create_spec();
        let mut current = create_spec();

        let mut base_op = operation();
        base_op.description = Some("Returns a list of users".to_string());
        base_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        let mut current_op = operation();
        current_op.description = Some("Returns a paginated list of users".to_string());
        current_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        add_path(&mut base, "/users", "get", base_op);
        add_path(&mut current, "/users", "get", current_op);

        let matcher = RouteMatcher::new(&base, &current);
        let results = matcher.match_routes();

        let route_result = results.iter().find(|r| r.name == "GET /users");
        assert!(route_result.is_some());

        let route = route_result.unwrap();
        let has_description_changed = route
            .violations
            .iter()
            .any(|v| v.name() == "RouteDescriptionChanged");
        assert!(
            has_description_changed,
            "Should detect description change"
        );
        assert_eq!(route.change_level, ChangeLevel::Change);
    }

    #[test]
    fn test_route_description_changed_rule_no_detection_when_empty() {
        // Test that adding description from empty is not detected
        let mut base_op = operation();
        base_op.summary = Some("Summary".to_string());
        // No description

        let mut current_op = operation();
        current_op.summary = Some("Summary".to_string());
        current_op.description = Some("New description added".to_string());

        let violations = RouteDescriptionChangedRule::detect(
            "/test",
            "get",
            Some(&base_op),
            Some(&current_op),
        );

        assert_eq!(
            violations.len(),
            0,
            "Should not detect adding a description from empty"
        );
    }

    #[test]
    fn test_required_parameter_added_rule_detection() {
        let mut base = create_spec();
        let mut current = create_spec();

        // Base has optional parameter
        let mut base_op = operation();
        base_op.parameters = vec![param!("page", ParameterIn::Query, false)];
        base_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        // Current has required parameter (new one)
        let mut current_op = operation();
        current_op.parameters = vec![
            param!("page", ParameterIn::Query, false),
            param!("sort", ParameterIn::Query, true), // New required param
        ];
        current_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        add_path(&mut base, "/users", "get", base_op);
        add_path(&mut current, "/users", "get", current_op);

        let matcher = RouteMatcher::new(&base, &current);
        let results = matcher.match_routes();

        let route_result = results.iter().find(|r| r.name == "GET /users");
        assert!(route_result.is_some());

        let route = route_result.unwrap();
        let required_violations: Vec<_> = route
            .violations
            .iter()
            .filter(|v| v.name() == "RequiredParameterAdded")
            .collect();

        assert!(
            !required_violations.is_empty(),
            "Should detect required parameter added"
        );
        assert_eq!(route.change_level, ChangeLevel::Breaking);
    }

    #[test]
    fn test_required_parameter_added_rule_optional_to_required() {
        // Test that making an optional parameter required is detected
        // The current implementation treats this as adding a new required parameter
        // because it uses (name, location) tuple comparison
        let mut base_op = operation();
        base_op.parameters = vec![param!("filter", ParameterIn::Query, false)];

        let mut current_op = operation();
        current_op.parameters = vec![param!("filter", ParameterIn::Query, true)];

        let violations = RequiredParameterAddedRule::detect(
            "/test",
            "get",
            Some(&base_op),
            Some(&current_op),
        );

        // Current implementation: optional parameter exists in base, so not detected as "new"
        // This is a known limitation - the rule only detects truly new required parameters
        // To detect optional->required transitions, we would need a separate rule
        assert_eq!(
            violations.len(),
            0,
            "Current implementation: only detects truly new required parameters, not optional->required transitions"
        );
    }

    #[test]
    fn test_required_parameter_added_rule_no_false_positive() {
        // Test that existing required parameters don't trigger detection
        let mut base_op = operation();
        base_op.parameters = vec![param!("id", ParameterIn::Path, true)];

        let mut current_op = operation();
        current_op.parameters = vec![param!("id", ParameterIn::Path, true)];

        let violations = RequiredParameterAddedRule::detect(
            "/test/{id}",
            "get",
            Some(&base_op),
            Some(&current_op),
        );

        assert_eq!(
            violations.len(),
            0,
            "Should not detect existing required parameters"
        );
    }

    #[test]
    fn test_parameter_removed_rule_detection() {
        let mut base = create_spec();
        let mut current = create_spec();

        let mut base_op = operation();
        base_op.parameters = vec![
            param!("page", ParameterIn::Query, false),
            param!("category", ParameterIn::Query, false),
        ];
        base_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        let mut current_op = operation();
        current_op.parameters = vec![param!("page", ParameterIn::Query, false)];
        current_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        add_path(&mut base, "/products", "get", base_op);
        add_path(&mut current, "/products", "get", current_op);

        let matcher = RouteMatcher::new(&base, &current);
        let results = matcher.match_routes();

        let route_result = results.iter().find(|r| r.name == "GET /products");
        assert!(route_result.is_some());

        let route = route_result.unwrap();
        let has_param_removed = route
            .violations
            .iter()
            .any(|v| v.name() == "ParameterRemoved" && v.description().contains("category"));
        assert!(has_param_removed, "Should detect parameter removal");
        assert_eq!(route.change_level, ChangeLevel::Breaking);
    }

    #[test]
    fn test_parameter_removed_rule_no_detection() {
        let mut base_op = operation();
        base_op.parameters = vec![param!("id", ParameterIn::Path, true)];

        let mut current_op = operation();
        current_op.parameters = vec![param!("id", ParameterIn::Path, true)];

        let violations =
            ParameterRemovedRule::detect("/test/{id}", "get", Some(&base_op), Some(&current_op));

        assert_eq!(
            violations.len(),
            0,
            "Should not detect when parameters are unchanged"
        );
    }

    #[test]
    fn test_parameter_location_distinction() {
        // Test that parameters are distinguished by both name and location
        let mut base_op = operation();
        base_op.parameters = vec![
            param!("id", ParameterIn::Query, false),
            param!("id", ParameterIn::Header, false),
            param!("filter", ParameterIn::Query, false),
        ];

        let mut current_op = operation();
        current_op.parameters = vec![
            param!("id", ParameterIn::Query, true), // Made required
            param!("id", ParameterIn::Header, false),
            param!("id", ParameterIn::Path, true), // New location
        ];

        // Should detect new "id" in path location
        let added_violations = RequiredParameterAddedRule::detect(
            "/test/{id}",
            "get",
            Some(&base_op),
            Some(&current_op),
        );

        let has_path_param = added_violations
            .iter()
            .any(|v| v.parameter_name == "id" && v.parameter_in.contains("Path"));
        assert!(
            has_path_param,
            "Should detect 'id' in path as new parameter"
        );

        // Should detect removed "filter" in query location
        let removed_violations =
            ParameterRemovedRule::detect("/test/{id}", "get", Some(&base_op), Some(&current_op));

        let has_filter_removed = removed_violations
            .iter()
            .any(|v| v.parameter_name == "filter");
        assert!(
            has_filter_removed,
            "Should detect 'filter' parameter removal"
        );
    }

    #[test]
    fn test_response_status_added_rule_detection() {
        let mut base = create_spec();
        let mut current = create_spec();

        let mut base_op = operation();
        base_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        let mut current_op = operation();
        current_op.responses = Some(BTreeMap::from([
            ("200".to_string(), response("OK")),
            ("400".to_string(), response("Bad Request")),
        ]));

        add_path(&mut base, "/users", "get", base_op);
        add_path(&mut current, "/users", "get", current_op);

        let matcher = RouteMatcher::new(&base, &current);
        let results = matcher.match_routes();

        let route_result = results.iter().find(|r| r.name == "GET /users");
        assert!(route_result.is_some());

        let route = route_result.unwrap();
        let has_status_added = route
            .violations
            .iter()
            .any(|v| v.name() == "ResponseStatusAdded" && v.description().contains("400"));
        assert!(has_status_added, "Should detect 400 status added");
        assert_eq!(route.change_level, ChangeLevel::Change);
    }

    #[test]
    fn test_response_status_added_rule_no_detection() {
        let mut base_op = operation();
        base_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        let mut current_op = operation();
        current_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        let violations = ResponseStatusAddedRule::detect(
            "/users",
            "get",
            Some(&base_op),
            Some(&current_op),
        );

        assert_eq!(violations.len(), 0, "Should not detect when responses unchanged");
    }

    #[test]
    fn test_response_status_removed_rule_detection() {
        let mut base = create_spec();
        let mut current = create_spec();

        let mut base_op = operation();
        base_op.responses = Some(BTreeMap::from([
            ("200".to_string(), response("OK")),
            ("404".to_string(), response("Not Found")),
        ]));

        let mut current_op = operation();
        current_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        add_path(&mut base, "/users/{id}", "get", base_op);
        add_path(&mut current, "/users/{id}", "get", current_op);

        let matcher = RouteMatcher::new(&base, &current);
        let results = matcher.match_routes();

        let route_result = results.iter().find(|r| r.name == "GET /users/{id}");
        assert!(route_result.is_some());

        let route = route_result.unwrap();
        let has_status_removed = route
            .violations
            .iter()
            .any(|v| v.name() == "ResponseStatusRemoved" && v.description().contains("404"));
        assert!(has_status_removed, "Should detect 404 status removed");
        assert_eq!(route.change_level, ChangeLevel::Warning);
    }

    #[test]
    fn test_response_status_removed_rule_no_detection() {
        let mut base_op = operation();
        base_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        let mut current_op = operation();
        current_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        let violations = ResponseStatusRemovedRule::detect(
            "/users",
            "get",
            Some(&base_op),
            Some(&current_op),
        );

        assert_eq!(violations.len(), 0, "Should not detect when responses unchanged");
    }

    #[test]
    fn test_multiple_violations_in_single_route() {
        let mut base = create_spec();
        let mut current = create_spec();

        let mut base_op = operation();
        base_op.summary = Some("Get users".to_string());
        base_op.description = Some("Returns users".to_string());
        base_op.parameters = vec![param!("page", ParameterIn::Query, false)];
        base_op.responses = Some(BTreeMap::from([
            ("200".to_string(), response("OK")),
            ("500".to_string(), response("Server Error")),
        ]));

        let mut current_op = operation();
        current_op.summary = Some("Get all users".to_string());
        current_op.description = Some("Returns paginated users".to_string());
        current_op.parameters = vec![
            param!("page", ParameterIn::Query, false),
            param!("sort", ParameterIn::Query, true), // New required
        ];
        current_op.responses = Some(BTreeMap::from([
            ("200".to_string(), response("OK")),
            ("400".to_string(), response("Bad Request")), // New
        ]));

        add_path(&mut base, "/users", "get", base_op);
        add_path(&mut current, "/users", "get", current_op);

        let matcher = RouteMatcher::new(&base, &current);
        let results = matcher.match_routes();

        let route_result = results.iter().find(|r| r.name == "GET /users");
        assert!(route_result.is_some());

        let route = route_result.unwrap();
        assert!(
            route.violations.len() >= 4,
            "Should have multiple violations (summary, description, parameter, response)"
        );
    }

    #[test]
    fn test_change_level_hierarchy() {
        // Test that Breaking takes precedence over Warning and Change
        let mut base = create_spec();
        let mut current = create_spec();

        let mut base_op = operation();
        base_op.summary = Some("Get user".to_string()); // Change level
        base_op.parameters = vec![param!("id", ParameterIn::Path, true)];
        base_op.responses = Some(BTreeMap::from([
            ("200".to_string(), response("OK")),
            ("404".to_string(), response("Not Found")), // Will be removed (Warning)
        ]));

        let mut current_op = operation();
        current_op.summary = Some("Get user by ID".to_string()); // Change level
        current_op.parameters = vec![];  // Removed parameter (Breaking)
        current_op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        add_path(&mut base, "/users/{id}", "get", base_op);
        add_path(&mut current, "/users/{id}", "get", current_op);

        let matcher = RouteMatcher::new(&base, &current);
        let results = matcher.match_routes();

        let route_result = results.iter().find(|r| r.name == "GET /users/{id}");
        assert!(route_result.is_some());

        let route = route_result.unwrap();
        // Should be Breaking because parameter removal is breaking
        assert_eq!(
            route.change_level,
            ChangeLevel::Breaking,
            "Breaking should take precedence"
        );

        // Should have at least one breaking violation
        let has_breaking = route
            .violations
            .iter()
            .any(|v| matches!(v.change_level(), ChangeLevel::Breaking));
        assert!(has_breaking, "Should have breaking violation");
    }

    #[test]
    fn test_no_false_positives_for_identical_routes() {
        let mut base = create_spec();
        let mut current = create_spec();

        let mut op = operation();
        op.summary = Some("Get users".to_string());
        op.description = Some("Returns all users".to_string());
        op.parameters = vec![param!("page", ParameterIn::Query, false)];
        op.responses = Some(BTreeMap::from([("200".to_string(), response("OK"))]));

        add_path(&mut base, "/users", "get", op.clone());
        add_path(&mut current, "/users", "get", op);

        let matcher = RouteMatcher::new(&base, &current);
        let results = matcher.match_routes();

        let route_result = results.iter().find(|r| r.name == "GET /users");
        // Either no result or empty violations
        if let Some(route) = route_result {
            assert_eq!(
                route.violations.len(),
                0,
                "Identical routes should have no violations"
            );
        }
    }

    #[test]
    fn test_all_http_methods_supported() {
        let mut base = create_spec();
        let mut current = create_spec();

        // Add operations for different HTTP methods in base
        add_path(&mut base, "/users", "get", operation());
        add_path(&mut base, "/users", "post", operation());
        add_path(&mut base, "/users/{id}", "put", operation());
        add_path(&mut base, "/users/{id}", "delete", operation());
        add_path(&mut base, "/users/{id}", "patch", operation());

        // Remove some in current
        add_path(&mut current, "/users", "get", operation());
        add_path(&mut current, "/users", "post", operation());

        let matcher = RouteMatcher::new(&base, &current);
        let results = matcher.match_routes();

        // Should detect all removed methods
        assert!(results.iter().any(|r| r.name.contains("DELETE")));
        assert!(results.iter().any(|r| r.name.contains("PUT")));
        assert!(results.iter().any(|r| r.name.contains("PATCH")));
    }
}
