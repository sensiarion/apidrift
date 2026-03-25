//! OpenAPI parse error formatting with context, path, and offending node.

use std::path::Path;

/// Extracts line and column from a serde/parser error message (e.g. "at line 1 column 48009").
pub fn parse_line_column(error_message: &str) -> Option<(usize, usize)> {
    let line_marker = "line ";
    let column_marker = "column ";
    let line_start = error_message.find(line_marker)? + line_marker.len();
    let line_slice = &error_message[line_start..];
    let line_end = line_slice.find(|c: char| !c.is_ascii_digit())?;
    let line: usize = line_slice[..line_end].parse().ok()?;

    let column_start = line_slice.find(column_marker)? + column_marker.len();
    let column_slice = &line_slice[column_start..];
    let column_end = column_slice
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(column_slice.len());
    let column: usize = column_slice[..column_end].parse().ok()?;

    Some((line, column))
}

/// Builds a snippet of source around the given line/column with a caret.
fn build_error_context(source: &str, line: usize, column: usize) -> Option<String> {
    let line_text = source.lines().nth(line.saturating_sub(1))?;
    let chars: Vec<char> = line_text.chars().collect();
    if chars.is_empty() {
        return None;
    }

    let index = column.saturating_sub(1).min(chars.len());
    let window = 80usize;
    let start = index.saturating_sub(window);
    let end = (index + window).min(chars.len());
    let snippet: String = chars[start..end].iter().collect();
    let prefix = if start > 0 { "..." } else { "" };
    let suffix = if end < chars.len() { "..." } else { "" };
    let caret_offset = index.saturating_sub(start) + prefix.len();

    Some(format!(
        "Context (line {}, column {}):\n{}{}{}\n{}^",
        line,
        column,
        prefix,
        snippet,
        suffix,
        " ".repeat(caret_offset)
    ))
}

fn truncate_for_display(text: &str, max_len: usize) -> String {
    if text.len() <= max_len {
        return text.to_string();
    }
    let mut truncated = text.chars().take(max_len).collect::<String>();
    truncated.push_str("\n... (truncated)");
    truncated
}

/// Converts a path like "paths./foo.get.parameters[0]" to a JSON pointer "/paths/~1foo/get/parameters/0".
fn path_to_json_pointer(path: &str) -> Option<String> {
    if path.is_empty() || path == "<unknown>" {
        return None;
    }

    let mut segments: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut chars = path.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            '.' => {
                if !current.is_empty() {
                    segments.push(current.clone());
                    current.clear();
                }
            }
            '[' => {
                if !current.is_empty() {
                    segments.push(current.clone());
                    current.clear();
                }
                let mut index = String::new();
                for next in chars.by_ref() {
                    if next == ']' {
                        break;
                    }
                    index.push(next);
                }
                if !index.is_empty() {
                    segments.push(index);
                }
            }
            _ => current.push(ch),
        }
    }

    if !current.is_empty() {
        segments.push(current);
    }

    if segments.is_empty() {
        return None;
    }

    let pointer = segments
        .into_iter()
        .map(|segment| segment.replace('~', "~0").replace('/', "~1"))
        .fold(String::new(), |mut acc, segment| {
            acc.push('/');
            acc.push_str(&segment);
            acc
        });

    Some(pointer)
}

/// Formats an OpenAPI parse error with optional context, error path, and offending JSON node.
///
/// - `path`: file path (for the message).
/// - `format_kind`: `"json"` or `"yaml"`.
/// - `err_message`: full error string from the parser.
/// - `source`: raw file content (used for context snippet and, for JSON, offending node).
/// - `error_path`: when present (e.g. from `serde_path_to_error`), used to show "Error path"
///   and "Offending node" for JSON.
pub fn format_openapi_parse_error(
    path: &Path,
    format_kind: &str,
    err_message: &str,
    source: &str,
    error_path: Option<&str>,
) -> String {
    let base = format!(
        "Invalid OpenAPI {} schema in \"{}\". Error: {}",
        format_kind.to_uppercase(),
        path.display(),
        err_message
    );

    let mut message = if let Some((line, column)) = parse_line_column(err_message) {
        if let Some(context) = build_error_context(source, line, column) {
            format!("{base}\n{context}")
        } else {
            base
        }
    } else {
        base
    };

    if let Some(ep) = error_path {
        if let Some(pointer) = path_to_json_pointer(ep) {
            if let Ok(value) = serde_json::from_str::<serde_json::Value>(source) {
                if let Some(node) = value.pointer(&pointer) {
                    if let Ok(pretty) = serde_json::to_string_pretty(node) {
                        let trimmed = truncate_for_display(&pretty, 2000);
                        message
                            .push_str(&format!("\nError path: {ep}\nOffending node:\n{trimmed}"));
                    }
                }
            }
        }
    }

    message
}
