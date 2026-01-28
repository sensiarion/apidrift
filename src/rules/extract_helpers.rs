pub fn extract_schema_name(
    schema: &oas3::spec::ObjectOrReference<oas3::spec::ObjectSchema>,
) -> Option<String> {
    match schema {
        oas3::spec::ObjectOrReference::Ref { ref_path, .. } => ref_path
            .strip_prefix("#/components/schemas/")
            .map(|s| s.to_string()),
        _ => None,
    }
}

// TODO extract responses
// TODO extract request
