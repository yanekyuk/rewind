use crate::domain::game::SteamDepotInfo;

/// Raw depot entry from the sidecar JSON, using the sidecar's field names.
#[derive(serde::Deserialize)]
struct RawDepotEntry {
    depot_id: u64,
    name: Option<String>,
    max_size: Option<u64>,
    dlc_app_id: Option<u64>,
}

/// Envelope for the sidecar's `depot_list` JSON message.
#[derive(serde::Deserialize)]
struct DepotListMessage {
    #[serde(default)]
    r#type: String,
    #[serde(default)]
    depots: Vec<RawDepotEntry>,
}

/// Parse SteamKit sidecar's depot listing output into a list of depot info.
///
/// The sidecar outputs newline-delimited JSON (NDJSON). The depot data
/// arrives in a message with `"type":"depot_list"` containing a `depots`
/// array. Other message types (log, done, etc.) are silently ignored.
///
/// Expected format:
/// ```json
/// {"type":"log","level":"info","message":"..."}
/// {"type":"depot_list","depots":[{"depot_id":3321461,"name":"Content","max_size":133575233011,"dlc_app_id":null}]}
/// {"type":"done","success":true}
/// ```
pub fn parse_depot_list(output: &str) -> Vec<SteamDepotInfo> {
    let mut entries = Vec::new();

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Ok(msg) = serde_json::from_str::<DepotListMessage>(trimmed) {
            if msg.r#type == "depot_list" && !msg.depots.is_empty() {
                entries.extend(msg.depots.into_iter().map(|raw| SteamDepotInfo {
                    depot_id: raw.depot_id.to_string(),
                    name: raw.name,
                    max_size: raw.max_size,
                    dlc_app_id: raw.dlc_app_id.map(|id| id.to_string()),
                }));
            }
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_depot_list_envelope() {
        let output = r#"{"type":"log","level":"info","message":"Connected"}
{"type":"depot_list","depots":[{"depot_id":3321461,"name":"Content","max_size":133575233011,"dlc_app_id":null},{"depot_id":3321462,"name":"DLC","max_size":5000000000,"dlc_app_id":3321470}]}
{"type":"done","success":true}"#;

        let entries = parse_depot_list(output);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].depot_id, "3321461");
        assert_eq!(entries[0].name, Some("Content".to_string()));
        assert_eq!(entries[0].max_size, Some(133_575_233_011));
        assert_eq!(entries[0].dlc_app_id, None);
        assert_eq!(entries[1].depot_id, "3321462");
        assert_eq!(entries[1].name, Some("DLC".to_string()));
        assert_eq!(entries[1].max_size, Some(5_000_000_000));
        assert_eq!(entries[1].dlc_app_id, Some("3321470".to_string()));
    }

    #[test]
    fn parse_depot_list_empty_output() {
        assert!(parse_depot_list("").is_empty());
    }

    #[test]
    fn parse_depot_list_empty_depots_array() {
        let output = r#"{"type":"depot_list","depots":[]}"#;
        assert!(parse_depot_list(output).is_empty());
    }

    #[test]
    fn parse_depot_list_ignores_non_depot_lines() {
        let output = r#"{"type":"log","level":"info","message":"hello"}
{"type":"done","success":true}
not json at all"#;
        assert!(parse_depot_list(output).is_empty());
    }

    #[test]
    fn parse_depot_list_with_all_optional_null() {
        let output = r#"{"type":"depot_list","depots":[{"depot_id":999,"name":null,"max_size":null,"dlc_app_id":null}]}"#;

        let entries = parse_depot_list(output);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].depot_id, "999");
        assert_eq!(entries[0].name, None);
        assert_eq!(entries[0].max_size, None);
        assert_eq!(entries[0].dlc_app_id, None);
    }

    #[test]
    fn parse_depot_list_converts_depot_id_to_string() {
        let output = r#"{"type":"depot_list","depots":[{"depot_id":12345678,"name":"Test","max_size":100,"dlc_app_id":null}]}"#;

        let entries = parse_depot_list(output);
        assert_eq!(entries[0].depot_id, "12345678");
    }
}
