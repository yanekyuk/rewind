use super::{VdfDocument, VdfMap, VdfValue};

/// Serialize a VdfDocument back to VDF text format.
pub fn serialize(doc: &VdfDocument) -> String {
    let mut output = String::new();
    write_quoted(&mut output, &doc.key);
    output.push('\n');
    match &doc.value {
        VdfValue::Map(map) => write_map(&mut output, map, 0),
        VdfValue::String(s) => {
            write_quoted(&mut output, s);
            output.push('\n');
        }
    }
    output
}

fn write_quoted(output: &mut String, s: &str) {
    output.push('"');
    for ch in s.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\t' => output.push_str("\\t"),
            other => output.push(other),
        }
    }
    output.push('"');
}

fn write_indent(output: &mut String, depth: usize) {
    for _ in 0..depth {
        output.push('\t');
    }
}

fn write_map(output: &mut String, map: &VdfMap, depth: usize) {
    write_indent(output, depth);
    output.push_str("{\n");
    for (key, value) in map {
        write_indent(output, depth + 1);
        write_quoted(output, key);
        match value {
            VdfValue::String(s) => {
                output.push_str("\t\t");
                write_quoted(output, s);
                output.push('\n');
            }
            VdfValue::Map(inner) => {
                output.push('\n');
                write_map(output, inner, depth + 1);
            }
        }
    }
    write_indent(output, depth);
    output.push_str("}\n");
}
