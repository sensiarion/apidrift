# Rules Architecture

## Overview

The OpenAPI Diff tool has been refactored to use a **Rule-based architecture** that separates the detection of differences from the business logic. This makes the system more extensible, maintainable, and easier to expand to support different API aspects (routes, parameters, responses, etc.).

## Key Concepts

### Rule Trait

The core abstraction is the `Rule` trait defined in `src/rules.rs`:

```rust
pub trait Rule: std::fmt::Debug {
    fn name(&self) -> &str;
    fn description(&self) -> String;
    fn change_level(&self) -> ChangeLevel;
    fn context(&self) -> String;
    fn category(&self) -> RuleCategory;
}
```

Every rule must implement this trait, providing:
- **name**: A unique identifier for the rule (e.g., "PropertyRemoved")
- **description**: Human-readable explanation of what was detected
- **change_level**: Severity level (Breaking, Warning, or Change)
- **context**: Where the violation occurred (e.g., "schema: User, property: email")
- **category**: The API aspect this rule applies to (Schema, Endpoint, Parameter, etc.)

### Rule Categories

Rules are organized by category:

```rust
pub enum RuleCategory {
    Schema,
    Endpoint,
    Parameter,
    Response,
    RequestBody,
}
```

This allows the system to easily expand to support:
- **Schema rules** - Changes in data models (currently implemented)
- **Endpoint rules** - Changes in API routes (future)
- **Parameter rules** - Changes in request parameters (future)
- **Response rules** - Changes in API responses (future)
- **RequestBody rules** - Changes in request bodies (future)

### Rule Violations

When a rule detects a difference, it creates a `RuleViolation`:

```rust
pub struct RuleViolation {
    rule: Box<dyn Rule>,
}
```

This wraps the concrete rule instance and provides convenient access methods.

### Match Results

The comparison process returns `MatchResult` objects:

```rust
pub struct MatchResult {
    pub name: String,
    pub violations: Vec<RuleViolation>,
    pub change_level: ChangeLevel,
}
```

Each result represents all rule violations found for a specific API element (e.g., a schema).

## Implemented Schema Rules

### Breaking Changes
- **SchemaRemovedRule** - A schema was deleted
- **TypeChangedRule** - A field's type changed
- **PropertyRemovedRule** - A property was deleted
- **RequiredPropertyAddedRule** - A new required property was added
- **EnumValuesRemovedRule** - Enum values were removed
- **NullableChangedRule** (nullable → non-nullable) - Field became required

### Warnings
- **FormatChangedRule** - Format constraint changed (e.g., email → uri)
- **NullableChangedRule** (non-nullable → nullable) - Field became optional

### Non-Breaking Changes
- **SchemaAddedRule** - A new schema was added
- **PropertyAddedRule** - A new optional property was added
- **RequiredPropertyRemovedRule** - A required property became optional
- **DescriptionChangedRule** - Documentation was updated
- **EnumValuesAddedRule** - New enum values were added

## Rule Inheritance Pattern

The architecture supports rule composition/inheritance. For example:

```rust
// Base rule for property removal
PropertyRemovedRule {
    schema_name: "User",
    property_path: "",
    property_name: "email",
}
// Change level: Breaking

// Specialized rule for required property removal
RequiredPropertyRemovedRule {
    schema_name: "User",
    property_path: "",
    property_name: "email",
}
// Change level: Change (less severe)
```

This allows specialized rules to override the change level or behavior without duplicating code.

## File Structure

```
src/
├── rules.rs              # Rule trait and core types
├── rules/
│   └── schema.rs         # Schema-specific rules
├── matcher.rs            # Schema comparison logic
├── render/
│   └── html.rs          # HTML report generation
└── lib.rs               # Public exports
```

## Adding New Rules

To add a new rule:

1. **Define the rule struct** in the appropriate module (e.g., `src/rules/schema.rs`):

```rust
#[derive(Debug, Clone)]
pub struct MyNewRule {
    pub schema_name: String,
    pub property_path: String,
    // ... other fields
}
```

2. **Implement the Rule trait**:

```rust
impl Rule for MyNewRule {
    fn name(&self) -> &str {
        "MyNewRule"
    }
    
    fn description(&self) -> String {
        format!("Custom description for {}", self.schema_name)
    }
    
    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Warning
    }
    
    fn context(&self) -> String {
        format!("schema: {}", self.schema_name)
    }
}
```

3. **Detect and create violations** in the matcher:

```rust
violations.push(RuleViolation::new(Box::new(MyNewRule {
    schema_name: schema_name.to_string(),
    property_path: property_path.to_string(),
})));
```

4. **Update renderers** if needed (e.g., add emoji mapping in `html.rs`).

## Extending to Routes

The architecture is designed to support route rules in the future:

```rust
// Future: src/rules/route.rs

#[derive(Debug, Clone)]
pub struct EndpointRemovedRule {
    pub path: String,
    pub method: String,
}

impl Rule for EndpointRemovedRule {
    fn name(&self) -> &str {
        "EndpointRemoved"
    }
    
    fn description(&self) -> String {
        format!("Endpoint {} {} was removed", self.method, self.path)
    }
    
    fn change_level(&self) -> ChangeLevel {
        ChangeLevel::Breaking
    }
    
    fn context(&self) -> String {
        format!("endpoint: {} {}", self.method, self.path)
    }
    
    fn category(&self) -> RuleCategory {
        RuleCategory::Endpoint
    }
}
```

Then create a `RouteMatcher` similar to `SchemaMatcher` that compares endpoints and creates route-specific violations.

## Benefits

1. **Extensibility**: Easy to add new rules without modifying core logic
2. **Separation of Concerns**: Detection, classification, and rendering are separate
3. **Type Safety**: Each rule is a distinct type with its own data
4. **Testability**: Rules can be tested individually
5. **Flexibility**: Rules can be filtered, grouped, or customized per use case
6. **Documentation**: Each rule is self-documenting through its name and description

## Migration Notes

The refactoring replaced the old `SchemaDifference` enum with the Rule-based system:

**Before**:
```rust
pub enum SchemaDifference {
    PropertyRemoved { property_name: String },
    // ...
}
```

**After**:
```rust
pub struct PropertyRemovedRule {
    pub schema_name: String,
    pub property_path: String,
    pub property_name: String,
}

impl Rule for PropertyRemovedRule { /* ... */ }
```

This provides more context (schema name, property path) and allows each rule to determine its own change level dynamically.

## Future Enhancements

Potential future improvements:

1. **Rule Configuration** - Allow users to customize change levels
2. **Rule Filtering** - Enable/disable specific rules
3. **Custom Rules** - Plugin system for user-defined rules
4. **Rule Groups** - Organize related rules together
5. **Rule Metrics** - Track which rules are triggered most often
6. **AI Integration** - Use LLMs to suggest rule classifications

