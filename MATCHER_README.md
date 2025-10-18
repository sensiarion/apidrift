# OpenAPI Matcher Implementation

## Overview

The matcher module provides functionality to compare OpenAPI schemas between two versions and detect differences with appropriate change levels.

## Features Implemented

### 1. Schema Comparison
- ✅ Matches schemas by name between base and current versions
- ✅ Detects added schemas (new models)
- ✅ Detects removed schemas (deleted models)
- ✅ Compares schema properties in detail
- ✅ **Resolves `$ref` references** (e.g., `#/components/schemas/User`)
- ✅ Handles both inline `Object` schemas and `Ref` variants
- ✅ Protection against infinite recursion on circular references

### 2. Difference Detection

The matcher can detect the following types of changes:

#### Breaking Changes
- **Removed Schema**: A schema that exists in base but not in current version
- **Type Changed**: Schema type has been modified (e.g., string → number)
- **Required Properties Added**: New properties marked as required
- **Property Removed**: Existing property has been removed
- **Enum Values Removed**: Enum values that were available are no longer present
- **Nullable Changed** (non-nullable → nullable is Breaking): Making a nullable field non-nullable

#### Warnings
- **Format Changed**: Field format has changed (e.g., email → uri)
- **Nullable Changed** (nullable → non-nullable is Warning): Making a field nullable when it wasn't before

#### Non-Breaking Changes
- **Added Schema**: New schema added (new model)
- **Required Properties Removed**: Properties are no longer required
- **Property Added**: New optional property added
- **Description Changed**: Documentation/description updated
- **Enum Values Added**: New enum values available

### 3. Change Level Calculation

The matcher automatically calculates the overall change level for each schema:
- If any difference is **Breaking**, the overall level is **Breaking**
- If any difference is **Warning** (and none are Breaking), the overall level is **Warning**
- Otherwise, the overall level is **Change**

## Usage

### Basic Example

```rust
use oas3::OpenApiV3Spec;
use openapi_diff::matcher::SchemaMatcher;

// Load OpenAPI specs
let base: OpenApiV3Spec = oas3::from_json(base_content)?;
let current: OpenApiV3Spec = oas3::from_json(current_content)?;

// Get schemas
let base_schemas = &base.components.as_ref().unwrap().schemas;
let current_schemas = &current.components.as_ref().unwrap().schemas;

// Create matcher and compare
let matcher = SchemaMatcher::new(base_schemas, current_schemas);
let results = matcher.match_schemas();

// Process results
for result in results {
    println!("Schema: {}", result.name);
    println!("Change Level: {:?}", result.change_level);
    for diff in &result.differences {
        println!("  - {:?}", diff);
    }
}
```

### Running the Example

```bash
cargo run --example test_matcher
```

## Test Coverage

The implementation includes comprehensive tests:

1. **test_schema_matcher_with_test_schemas**: Overall integration test
2. **test_user_schema_changes**: Tests breaking changes (added required properties, removed properties)
3. **test_product_schema_changes**: Tests non-breaking changes (removed required properties, added enum values)
4. **test_status_enum_breaking_change**: Tests enum value removal (breaking change)
5. **test_new_model_added**: Tests detection of new schemas
6. **test_order_array_items_changes**: Tests array items detection
7. **test_change_level_hierarchy**: Tests change level priority

Run tests with:
```bash
cargo test
```

## Test Schemas

Sample schemas are provided in `tests/`:
- `base_test_schema.json`: Base version of API
- `current_test_schema.json`: Current version with various changes

These demonstrate:
- Breaking changes: User schema (added required field, removed field)
- Breaking changes: StatusEnum (removed enum value)
- Non-breaking changes: Product schema (removed required constraint, added enum values)
- Non-breaking changes: NewModel (new schema added)

## Current Limitations

1. **Array Items Comparison**: Nested array item schema comparison is not yet fully implemented (marked with TODO).
2. **AllOf/AnyOf/OneOf**: Schema composition keywords are not yet compared.
3. **Recursion Depth**: Schema comparison is limited to 10 levels of nesting to prevent stack overflow on circular references.

## Future Enhancements

- Full `$ref` resolution and comparison
- Array items deep comparison
- Schema composition comparison (allOf, anyOf, oneOf)
- Additional validation rules (min/max length, patterns, etc.)
- More granular change classification

## Module Structure

```
src/
├── lib.rs              # Library root, exports ChangeLevel
├── main.rs             # Binary entry point
└── matcher.rs          # Matcher implementation
    ├── SchemaDifference   # Enum of all possible differences
    ├── SchemaMatchResult  # Result of comparing one schema
    └── SchemaMatcher      # Main matcher struct

tests/
├── matcher_tests.rs    # Integration tests
├── base_test_schema.json    # Test data (base version)
└── current_test_schema.json # Test data (current version)

examples/
└── test_matcher.rs     # Example usage
```

## Implementation Details

### Type Mapping

The implementation correctly maps to the `oas3` crate types:
- Uses `BTreeMap<String, ObjectOrReference<ObjectSchema>>` for schemas
- Uses `ObjectSchema` directly (not boxed in the map)
- Uses `TypeSet` for schema types
- Uses `enum_values` field (not `enumeration`)
- Uses `ObjectSchema::is_nullable()` method for nullable detection

### Change Level Logic

Change levels follow OpenAPI diff best practices:
- **Breaking**: Changes that could break existing clients (removed fields, changed types, new required fields)
- **Warning**: Changes that might cause issues (format changes, nullability changes)
- **Change**: Safe additions and relaxations (new optional fields, new enum values, removed requirements)

