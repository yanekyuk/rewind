mod parser;
mod serializer;
mod acf;

pub use parser::parse;
pub use serializer::serialize;
pub use acf::{AppState, InstalledDepot};

/// A VDF value is either a string or a nested map of key-value pairs.
/// Uses a Vec of pairs instead of a map to preserve insertion order
/// to support duplicate keys and preserve ordering.
#[derive(Debug, Clone, PartialEq)]
pub enum VdfValue {
    String(String),
    Map(VdfMap),
}

/// An ordered list of key-value pairs. Uses Vec to preserve insertion order
/// and support duplicate keys (which VDF technically allows).
pub type VdfMap = Vec<(String, VdfValue)>;

/// A parsed VDF document: a root key with a value (typically a Map).
#[derive(Debug, Clone, PartialEq)]
pub struct VdfDocument {
    pub key: String,
    pub value: VdfValue,
}

/// Errors that can occur during VDF parsing.
#[derive(Debug, Clone, PartialEq)]
pub enum VdfError {
    /// The input could not be parsed as valid VDF.
    ParseError(String),
    /// An ACF field is missing or has the wrong type.
    MissingField(String),
    /// An ACF field value could not be interpreted.
    InvalidField { field: String, detail: String },
}

impl std::fmt::Display for VdfError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            VdfError::ParseError(msg) => write!(f, "VDF parse error: {}", msg),
            VdfError::MissingField(field) => write!(f, "missing required field: {}", field),
            VdfError::InvalidField { field, detail } => {
                write!(f, "invalid field '{}': {}", field, detail)
            }
        }
    }
}

impl std::error::Error for VdfError {}

/// Helper to look up a value by key in a VdfMap.
pub fn map_get<'a>(map: &'a VdfMap, key: &str) -> Option<&'a VdfValue> {
    map.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Helper to get a string value by key from a VdfMap.
pub fn map_get_str<'a>(map: &'a VdfMap, key: &str) -> Option<&'a str> {
    match map_get(map, key) {
        Some(VdfValue::String(s)) => Some(s.as_str()),
        _ => None,
    }
}

/// Helper to get a nested map by key from a VdfMap.
pub fn map_get_map<'a>(map: &'a VdfMap, key: &str) -> Option<&'a VdfMap> {
    match map_get(map, key) {
        Some(VdfValue::Map(m)) => Some(m),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_key_value_pairs() {
        let input = r#""AppState"
{
    "appid"    "3321460"
    "name"     "Crimson Desert"
}"#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.key, "AppState");
        match &doc.value {
            VdfValue::Map(map) => {
                assert_eq!(map_get_str(map, "appid"), Some("3321460"));
                assert_eq!(map_get_str(map, "name"), Some("Crimson Desert"));
            }
            _ => panic!("expected Map"),
        }
    }

    #[test]
    fn parse_nested_structures() {
        let input = r#""AppState"
{
    "appid"    "3321460"
    "InstalledDepots"
    {
        "3321461"
        {
            "manifest"  "7446650175280810671"
            "size"      "133575233011"
        }
    }
}"#;
        let doc = parse(input).unwrap();
        let map = match &doc.value {
            VdfValue::Map(m) => m,
            _ => panic!("expected Map"),
        };
        let depots = map_get_map(map, "InstalledDepots").expect("InstalledDepots");
        let depot = map_get_map(depots, "3321461").expect("depot 3321461");
        assert_eq!(map_get_str(depot, "manifest"), Some("7446650175280810671"));
        assert_eq!(map_get_str(depot, "size"), Some("133575233011"));
    }

    #[test]
    fn parse_deeply_nested() {
        let input = r#""L1"
{
    "L2"
    {
        "L3"
        {
            "L4"
            {
                "L5"
                {
                    "deep"  "value"
                }
            }
        }
    }
}"#;
        let doc = parse(input).unwrap();
        let m1 = match &doc.value { VdfValue::Map(m) => m, _ => panic!() };
        let m2 = map_get_map(m1, "L2").unwrap();
        let m3 = map_get_map(m2, "L3").unwrap();
        let m4 = map_get_map(m3, "L4").unwrap();
        let m5 = map_get_map(m4, "L5").unwrap();
        assert_eq!(map_get_str(m5, "deep"), Some("value"));
    }

    #[test]
    fn parse_error_unclosed_brace() {
        let input = r#""Root"
{
    "key" "value"
"#;
        let result = parse(input);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            VdfError::ParseError(_) => {}
            _ => panic!("expected ParseError, got {:?}", err),
        }
    }

    #[test]
    fn parse_error_unclosed_quote() {
        let input = r#""Root"
{
    "key"  "value with no end
}"#;
        let result = parse(input);
        assert!(result.is_err());
    }

    #[test]
    fn parse_with_comments() {
        let input = r#"// This is a comment
"Root"
{
    // Another comment
    "key"  "value"
}"#;
        let doc = parse(input).unwrap();
        assert_eq!(doc.key, "Root");
        let map = match &doc.value { VdfValue::Map(m) => m, _ => panic!() };
        assert_eq!(map_get_str(map, "key"), Some("value"));
    }

    #[test]
    fn serialize_simple() {
        let doc = VdfDocument {
            key: "Root".into(),
            value: VdfValue::Map(vec![
                ("key1".into(), VdfValue::String("value1".into())),
                ("key2".into(), VdfValue::String("value2".into())),
            ]),
        };
        let output = serialize(&doc);
        let reparsed = parse(&output).unwrap();
        assert_eq!(doc, reparsed);
    }

    #[test]
    fn serialize_nested() {
        let doc = VdfDocument {
            key: "Root".into(),
            value: VdfValue::Map(vec![
                ("name".into(), VdfValue::String("test".into())),
                ("inner".into(), VdfValue::Map(vec![
                    ("a".into(), VdfValue::String("1".into())),
                    ("b".into(), VdfValue::String("2".into())),
                ])),
            ]),
        };
        let output = serialize(&doc);
        assert!(output.contains("\"inner\""));
        assert!(output.contains("\"a\"\t\t\"1\""));
        let reparsed = parse(&output).unwrap();
        assert_eq!(doc, reparsed);
    }

    #[test]
    fn round_trip_full_acf() {
        let input = r#""AppState"
{
    "appid"        "3321460"
    "name"         "Crimson Desert"
    "buildid"      "22560074"
    "installdir"   "Crimson Desert"
    "StateFlags"   "4"
    "InstalledDepots"
    {
        "3321461"
        {
            "manifest"  "7446650175280810671"
            "size"      "133575233011"
        }
    }
}"#;
        let doc = parse(input).unwrap();
        let output = serialize(&doc);
        let reparsed = parse(&output).unwrap();
        assert_eq!(doc, reparsed);
    }

    #[test]
    fn acf_from_vdf_basic() {
        let input = r#""AppState"
{
    "appid"        "3321460"
    "name"         "Crimson Desert"
    "buildid"      "22560074"
    "installdir"   "Crimson Desert"
    "StateFlags"   "4"
    "InstalledDepots"
    {
        "3321461"
        {
            "manifest"  "7446650175280810671"
            "size"      "133575233011"
        }
    }
}"#;
        let doc = parse(input).unwrap();
        let app = AppState::from_vdf(&doc).unwrap();
        assert_eq!(app.appid, "3321460");
        assert_eq!(app.name, "Crimson Desert");
        assert_eq!(app.buildid, "22560074");
        assert_eq!(app.installdir, "Crimson Desert");
        assert_eq!(app.state_flags, "4");
        assert_eq!(app.installed_depots.len(), 1);
        let depot = app.installed_depots.get("3321461").unwrap();
        assert_eq!(depot.manifest, "7446650175280810671");
        assert_eq!(depot.size, "133575233011");
        assert_eq!(app.target_build_id, None);
        assert_eq!(app.bytes_to_download, None);
    }

    #[test]
    fn acf_with_optional_fields() {
        let input = r#""AppState"
{
    "appid"            "3321460"
    "name"             "Crimson Desert"
    "buildid"          "22560074"
    "installdir"       "Crimson Desert"
    "StateFlags"       "4"
    "TargetBuildID"    "22570000"
    "BytesToDownload"  "5000000"
    "InstalledDepots"
    {
        "3321461"
        {
            "manifest"  "7446650175280810671"
            "size"      "133575233011"
        }
    }
}"#;
        let doc = parse(input).unwrap();
        let app = AppState::from_vdf(&doc).unwrap();
        assert_eq!(app.target_build_id, Some("22570000".to_string()));
        assert_eq!(app.bytes_to_download, Some("5000000".to_string()));
    }

    #[test]
    fn acf_multiple_depots() {
        let input = r#""AppState"
{
    "appid"        "12345"
    "name"         "Multi Depot Game"
    "buildid"      "100"
    "installdir"   "MultiDepot"
    "StateFlags"   "4"
    "InstalledDepots"
    {
        "12346"
        {
            "manifest"  "111111"
            "size"      "1000"
        }
        "12347"
        {
            "manifest"  "222222"
            "size"      "2000"
        }
    }
}"#;
        let doc = parse(input).unwrap();
        let app = AppState::from_vdf(&doc).unwrap();
        assert_eq!(app.installed_depots.len(), 2);
        assert_eq!(app.installed_depots["12346"].manifest, "111111");
        assert_eq!(app.installed_depots["12347"].manifest, "222222");
    }

    #[test]
    fn acf_missing_required_field() {
        let input = r#""AppState"
{
    "appid"  "123"
}"#;
        let doc = parse(input).unwrap();
        let result = AppState::from_vdf(&doc);
        assert!(result.is_err());
        match result.unwrap_err() {
            VdfError::MissingField(f) => assert_eq!(f, "name"),
            e => panic!("expected MissingField, got {:?}", e),
        }
    }

    #[test]
    fn acf_wrong_root_key() {
        let input = r#""NotAppState"
{
    "appid"  "123"
}"#;
        let doc = parse(input).unwrap();
        let result = AppState::from_vdf(&doc);
        assert!(result.is_err());
    }

    #[test]
    fn acf_round_trip_via_to_vdf() {
        let app = AppState {
            appid: "3321460".into(),
            name: "Crimson Desert".into(),
            buildid: "22560074".into(),
            installdir: "Crimson Desert".into(),
            state_flags: "4".into(),
            installed_depots: {
                let mut m = std::collections::HashMap::new();
                m.insert("3321461".into(), InstalledDepot {
                    manifest: "7446650175280810671".into(),
                    size: "133575233011".into(),
                });
                m
            },
            target_build_id: None,
            bytes_to_download: None,
            full_validate_after_next_update: None,
        };
        let doc = app.to_vdf();
        let serialized = serialize(&doc);
        let reparsed_doc = parse(&serialized).unwrap();
        let reparsed_app = AppState::from_vdf(&reparsed_doc).unwrap();
        assert_eq!(app, reparsed_app);
    }
}
