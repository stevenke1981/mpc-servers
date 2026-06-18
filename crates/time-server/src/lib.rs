use std::sync::Arc;

use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Offset, TimeZone, Utc};
use chrono_tz::Tz;
use rmcp::{
    handler::server::ServerHandler,
    model::{
        CallToolRequestParams, CallToolResult, ErrorCode, Implementation, ListToolsResult,
        PaginatedRequestParams, ServerCapabilities, ServerInfo, Tool, ToolAnnotations,
        ToolsCapability,
    },
    service::{RequestContext, RoleServer},
    ErrorData as McpError,
};

/// Concrete timezone offset in seconds east of UTC.
type OffsetSecs = i32;

/// Check if Daylight Saving Time is in effect for a given timezone at a given UTC moment.
///
/// DST always adds a positive amount to the UTC offset, so the standard (non-DST) offset
/// is the minimum of the January and July offsets.
fn is_dst_in_effect(tz: &Tz, utc_naive: &NaiveDateTime) -> bool {
    let current_secs = offset_at_utc(tz, utc_naive);

    let jan = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(utc_naive.year(), 1, 1).unwrap(),
        NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
    );
    let jul = NaiveDateTime::new(
        NaiveDate::from_ymd_opt(utc_naive.year(), 7, 1).unwrap(),
        NaiveTime::from_hms_opt(12, 0, 0).unwrap(),
    );

    let std_secs = offset_at_utc(tz, &jan).min(offset_at_utc(tz, &jul));
    current_secs != std_secs
}

fn offset_at_utc(tz: &Tz, utc_dt: &NaiveDateTime) -> OffsetSecs {
    tz.offset_from_utc_datetime(utc_dt).fix().local_minus_utc()
}

fn resolve_tz(name: &str) -> Result<Tz, McpError> {
    name.parse::<Tz>()
        .map_err(|_| McpError::invalid_params(format!("Invalid timezone: '{name}'"), None))
}

fn parse_datetime(s: &str) -> Result<NaiveDateTime, McpError> {
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S"))
        .map_err(|_| {
            McpError::invalid_params(
                format!(
                    "Cannot parse datetime '{s}'. Expected ISO 8601 (YYYY-MM-DDTHH:MM:SS) \
                     or space-separated (YYYY-MM-DD HH:MM:SS)"
                ),
                None,
            )
        })
}

// ---------------------------------------------------------------------------
// Tool schemas
// ---------------------------------------------------------------------------

fn build_get_current_time_schema() -> Arc<rmcp::model::JsonObject> {
    Arc::new(
        if let serde_json::Value::Object(obj) = serde_json::json!({
            "type": "object",
            "properties": {
                "timezone": {
                    "type": "string",
                    "description": "IANA timezone name (e.g., America/New_York, Asia/Taipei)"
                }
            },
            "required": ["timezone"]
        }) {
            obj
        } else {
            unreachable!()
        },
    )
}

fn build_convert_time_schema() -> Arc<rmcp::model::JsonObject> {
    Arc::new(
        if let serde_json::Value::Object(obj) = serde_json::json!({
            "type": "object",
            "properties": {
                "source_timezone": {
                    "type": "string",
                    "description": "Source IANA timezone name (e.g., America/New_York)"
                },
                "source_datetime": {
                    "type": "string",
                    "description": "Source datetime in ISO 8601 (YYYY-MM-DDTHH:MM:SS) or space-separated (YYYY-MM-DD HH:MM:SS)"
                },
                "target_timezones": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Target IANA timezone name(s). Can be a single string or array of strings.",
                    "minItems": 1
                }
            },
            "required": ["source_timezone", "source_datetime", "target_timezones"]
        }) {
            obj
        } else {
            unreachable!()
        },
    )
}

fn build_get_current_time_tool() -> Tool {
    Tool::new(
        "get_current_time",
        "Get the current time in a specified timezone",
        build_get_current_time_schema(),
    )
    .with_annotations(
        ToolAnnotations::new()
            .read_only(true)
            .destructive(false)
            .idempotent(true)
            .open_world(false),
    )
}

fn build_convert_time_tool() -> Tool {
    Tool::new(
        "convert_time",
        "Convert a datetime between timezones. Supports multiple target timezones.",
        build_convert_time_schema(),
    )
    .with_annotations(
        ToolAnnotations::new()
            .read_only(true)
            .destructive(false)
            .idempotent(true)
            .open_world(false),
    )
}

// ---------------------------------------------------------------------------
// Implementation helpers
// ---------------------------------------------------------------------------

fn get_current_time_impl(tz_name: &str) -> Result<serde_json::Value, McpError> {
    let tz = resolve_tz(tz_name)?;
    let now_utc = Utc::now().naive_utc();
    let dt = tz.from_utc_datetime(&now_utc);

    Ok(serde_json::json!({
        "timezone": tz_name,
        "datetime": dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string(),
        "day_of_week": dt.format("%A").to_string(),
        "is_dst": is_dst_in_effect(&tz, &now_utc),
    }))
}

fn convert_time_impl(
    source_tz_name: &str,
    source_dt_str: &str,
    target_tzs_raw: &serde_json::Value,
) -> Result<serde_json::Value, McpError> {
    let src_tz = resolve_tz(source_tz_name)?;

    let naive_dt = parse_datetime(source_dt_str)?;

    // Interpret the naive datetime in the source timezone.
    // Use .earliest() to handle DST ambiguity.
    let src_dt = src_tz
        .from_local_datetime(&naive_dt)
        .earliest()
        .ok_or_else(|| {
            McpError::invalid_params(
                format!(
                    "Datetime '{source_dt_str}' is non-existent in timezone '{source_tz_name}' \
                     due to DST gap"
                ),
                None,
            )
        })?;

    // Parse target timezones: string or array of strings.
    let target_names: Vec<String> = match target_tzs_raw {
        serde_json::Value::String(s) => vec![s.clone()],
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => {
            return Err(McpError::invalid_params(
                "'target_timezones' must be a string or array of strings",
                None,
            ));
        }
    };

    if target_names.is_empty() {
        return Err(McpError::invalid_params(
            "'target_timezones' must contain at least one timezone",
            None,
        ));
    }

    let src_offset_secs = offset_at_utc(&src_tz, &src_dt.naive_utc());
    let mut targets = Vec::new();
    let mut differences = Vec::new();

    for target_name in &target_names {
        let tgt_tz = resolve_tz(target_name)?;
        let tgt_dt = src_dt.with_timezone(&tgt_tz);
        let tgt_offset_secs = offset_at_utc(&tgt_tz, &tgt_dt.naive_utc());

        targets.push(serde_json::json!({
            "timezone": target_name,
            "datetime": tgt_dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string(),
            "day_of_week": tgt_dt.format("%A").to_string(),
            "is_dst": is_dst_in_effect(&tgt_tz, &tgt_dt.naive_utc()),
        }));

        differences.push(serde_json::json!({
            "from": source_tz_name,
            "to": target_name,
            "difference_seconds": tgt_offset_secs - src_offset_secs,
        }));
    }

    Ok(serde_json::json!({
        "source": {
            "timezone": source_tz_name,
            "datetime": src_dt.format("%Y-%m-%dT%H:%M:%S%:z").to_string(),
            "day_of_week": src_dt.format("%A").to_string(),
            "is_dst": is_dst_in_effect(&src_tz, &src_dt.naive_utc()),
        },
        "targets": targets,
        "time_differences": differences,
    }))
}

// ---------------------------------------------------------------------------
// MCP handler
// ---------------------------------------------------------------------------

#[derive(Debug, Default)]
pub struct TimeServer;

impl ServerHandler for TimeServer {
    fn get_info(&self) -> ServerInfo {
        let mut caps = ServerCapabilities::default();
        caps.tools = Some(ToolsCapability { list_changed: None });
        ServerInfo::new(caps).with_server_info(Implementation::new(
            "time-server",
            env!("CARGO_PKG_VERSION"),
        ))
    }

    fn get_tool(&self, name: &str) -> Option<Tool> {
        match name {
            "get_current_time" => Some(build_get_current_time_tool()),
            "convert_time" => Some(build_convert_time_tool()),
            _ => None,
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParams>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, McpError> {
        Ok(ListToolsResult::with_all_items(vec![
            build_get_current_time_tool(),
            build_convert_time_tool(),
        ]))
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParams,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let args = request.arguments.as_ref();

        match request.name.as_ref() {
            "get_current_time" => {
                let tz_name = args
                    .and_then(|a| a.get("timezone"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpError::invalid_params("Missing required argument: 'timezone'", None)
                    })?;

                let result = get_current_time_impl(tz_name)?;
                Ok(CallToolResult::structured(result))
            }

            "convert_time" => {
                let source_tz_name = args
                    .and_then(|a| a.get("source_timezone"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpError::invalid_params(
                            "Missing required argument: 'source_timezone'",
                            None,
                        )
                    })?;

                let source_dt_str = args
                    .and_then(|a| a.get("source_datetime"))
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        McpError::invalid_params(
                            "Missing required argument: 'source_datetime'",
                            None,
                        )
                    })?;

                let target_tzs_raw =
                    args.and_then(|a| a.get("target_timezones"))
                        .ok_or_else(|| {
                            McpError::invalid_params(
                                "Missing required argument: 'target_timezones'",
                                None,
                            )
                        })?;

                let result = convert_time_impl(source_tz_name, source_dt_str, target_tzs_raw)?;
                Ok(CallToolResult::structured(result))
            }

            _ => Err(McpError::new(
                ErrorCode::METHOD_NOT_FOUND,
                format!("Unknown tool: {}", request.name),
                None,
            )),
        }
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;

    #[test]
    fn test_get_current_time_valid_tz() {
        let result = get_current_time_impl("Europe/Warsaw").unwrap();
        assert_eq!(result["timezone"], "Europe/Warsaw");
        let dt = result["datetime"].as_str().unwrap();
        assert!(
            dt.contains('T'),
            "datetime should contain T separator: {dt}"
        );
        assert!(
            dt.contains('+') || dt.ends_with('Z'),
            "datetime should have timezone offset: {dt}"
        );
        assert!(
            [
                "Monday",
                "Tuesday",
                "Wednesday",
                "Thursday",
                "Friday",
                "Saturday",
                "Sunday"
            ]
            .contains(&result["day_of_week"].as_str().unwrap()),
            "day_of_week should be a valid day name"
        );
    }

    #[test]
    fn test_get_current_time_invalid_timezone() {
        let err = get_current_time_impl("Invalid/Timezone").unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("Invalid timezone"), "error msg: {msg}");
    }

    #[test]
    fn test_convert_time_basic() {
        let result = convert_time_impl(
            "Europe/Warsaw",
            "2024-06-18T12:00:00",
            &serde_json::json!("Europe/London"),
        )
        .unwrap();
        let targets = result["targets"].as_array().unwrap();
        assert_eq!(targets.len(), 1);
        assert_eq!(targets[0]["timezone"], "Europe/London");
    }

    #[test]
    fn test_convert_time_multi_target() {
        let result = convert_time_impl(
            "America/New_York",
            "2024-06-18T12:00:00",
            &serde_json::json!(["Europe/London", "Asia/Tokyo"]),
        )
        .unwrap();
        let targets = result["targets"].as_array().unwrap();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0]["timezone"], "Europe/London");
        assert_eq!(targets[1]["timezone"], "Asia/Tokyo");
        let diffs = result["time_differences"].as_array().unwrap();
        assert_eq!(diffs.len(), 2);
    }

    #[test]
    fn test_convert_time_invalid_source_tz() {
        let err = convert_time_impl(
            "Invalid/Tz",
            "2024-06-18T12:00:00",
            &serde_json::json!("Europe/London"),
        )
        .unwrap_err();
        assert!(format!("{err}").contains("Invalid timezone"));
    }

    #[test]
    fn test_convert_time_invalid_target_tz() {
        let err = convert_time_impl(
            "Europe/Warsaw",
            "2024-06-18T12:00:00",
            &serde_json::json!("Invalid/Tz"),
        )
        .unwrap_err();
        assert!(format!("{err}").contains("Invalid timezone"));
    }

    #[test]
    fn test_convert_time_invalid_datetime_format() {
        let err = convert_time_impl(
            "Europe/Warsaw",
            "not-a-datetime",
            &serde_json::json!("Europe/London"),
        )
        .unwrap_err();
        assert!(format!("{err}").contains("datetime"));
    }

    #[test]
    fn test_convert_time_nepal_fractional() {
        let result = convert_time_impl(
            "Europe/Warsaw",
            "2024-06-18T12:00:00",
            &serde_json::json!("Asia/Kathmandu"),
        )
        .unwrap();
        let diffs = result["time_differences"].as_array().unwrap();
        let seconds = diffs[0]["difference_seconds"].as_i64().unwrap();
        // Asia/Kathmandu is UTC+5:45 (20700 sec), Europe/Warsaw in June is UTC+2 (7200 sec)
        // Difference = 20700 - 7200 = 13500 sec = 3.75h
        assert_eq!(
            seconds, 13500,
            "Kathmandu should be 3.75h ahead of Warsaw in June"
        );
    }

    #[test]
    fn test_resolve_tz_valid() {
        assert!(resolve_tz("UTC").is_ok());
        assert!(resolve_tz("America/New_York").is_ok());
        assert!(resolve_tz("Asia/Taipei").is_ok());
    }

    #[test]
    fn test_resolve_tz_invalid() {
        let err = resolve_tz("Not/A_Timezone").unwrap_err();
        assert!(format!("{err}").contains("Invalid timezone"));
    }

    #[test]
    fn test_tool_names() {
        let tool1 = build_get_current_time_tool();
        let tool2 = build_convert_time_tool();
        assert_eq!(tool1.name.as_ref(), "get_current_time");
        assert_eq!(tool2.name.as_ref(), "convert_time");
    }

    #[test]
    fn test_parse_datetime_iso() {
        let dt = parse_datetime("2024-06-18T12:30:00").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.month(), 6);
        assert_eq!(dt.day(), 18);
        assert_eq!(dt.hour(), 12);
        assert_eq!(dt.minute(), 30);
    }

    #[test]
    fn test_parse_datetime_space() {
        let dt = parse_datetime("2024-06-18 12:30:00").unwrap();
        assert_eq!(dt.year(), 2024);
        assert_eq!(dt.hour(), 12);
    }

    #[test]
    fn test_parse_datetime_invalid() {
        let err = parse_datetime("not-a-date").unwrap_err();
        assert!(format!("{err}").contains("Cannot parse datetime"));
    }

    #[test]
    fn test_convert_time_multiple_string_acceptance() {
        // Accept both single string and array
        let result_single = convert_time_impl(
            "UTC",
            "2024-06-18T12:00:00",
            &serde_json::json!("Asia/Taipei"),
        )
        .unwrap();
        let targets_single = result_single["targets"].as_array().unwrap();
        assert_eq!(targets_single[0]["timezone"], "Asia/Taipei");

        let result_array = convert_time_impl(
            "UTC",
            "2024-06-18T12:00:00",
            &serde_json::json!(["Asia/Taipei", "Europe/London"]),
        )
        .unwrap();
        let targets_array = result_array["targets"].as_array().unwrap();
        assert_eq!(targets_array.len(), 2);
    }

    #[test]
    fn test_make_tools_schema_no_boolean_nodes() {
        for tool in &[build_get_current_time_tool(), build_convert_time_tool()] {
            let schema_val = serde_json::Value::Object((*tool.input_schema).clone());
            if let Some(props) = schema_val["properties"].as_object() {
                for (key, val) in props {
                    assert!(
                        !val.is_boolean(),
                        "property '{key}' in tool '{}' is a bare boolean: {val}",
                        tool.name
                    );
                }
            }
        }
    }
}
