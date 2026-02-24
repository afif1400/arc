//! Arc MCP (Model Context Protocol) server — enables AI agents to
//! validate, render, and convert arc diagrams via JSON-RPC over stdio.

use serde::{Deserialize, Serialize};
use std::io::{self, BufRead, Write};

use crate::fmt;
use crate::layout;
use crate::parser;
use crate::svg;
use crate::themes;
use crate::validator;

// ── MCP Protocol Types ──────────────────────────────────────────

#[derive(Deserialize)]
#[allow(dead_code)]
struct JsonRpcRequest {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    method: String,
    #[serde(default)]
    params: serde_json::Value,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

// ── Tool definitions ────────────────────────────────────────────

fn tools_list() -> serde_json::Value {
    serde_json::json!({
        "tools": [
            {
                "name": "arc_validate",
                "description": "Validate arc diagram syntax. Returns structured errors with fix suggestions. Use this before rendering to check for issues.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "Arc diagram source code"
                        }
                    },
                    "required": ["code"]
                }
            },
            {
                "name": "arc_render",
                "description": "Render an arc diagram to SVG. Returns the SVG as a string.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "Arc diagram source code"
                        },
                        "theme": {
                            "type": "string",
                            "description": "Theme name: light, dark, blueprint, mono",
                            "default": "light"
                        }
                    },
                    "required": ["code"]
                }
            },
            {
                "name": "arc_format",
                "description": "Format/prettify arc diagram source code with consistent style.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "code": {
                            "type": "string",
                            "description": "Arc diagram source code to format"
                        }
                    },
                    "required": ["code"]
                }
            },
            {
                "name": "arc_grammar",
                "description": "Returns the complete arc grammar specification. Include this in your system prompt to generate valid arc diagrams.",
                "inputSchema": {
                    "type": "object",
                    "properties": {}
                }
            }
        ]
    })
}

// ── Tool execution ──────────────────────────────────────────────

fn execute_tool(name: &str, args: &serde_json::Value) -> Result<serde_json::Value, String> {
    match name {
        "arc_validate" => {
            let code = args
                .get("code")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'code' parameter")?;
            let parse_result = parser::parse(code);
            let (validation, _resolved) =
                validator::validate(&parse_result.document, &parse_result.diagnostics);
            Ok(serde_json::to_value(&validation).unwrap())
        }
        "arc_render" => {
            let code = args
                .get("code")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'code' parameter")?;
            let theme_name = args
                .get("theme")
                .and_then(|v| v.as_str())
                .unwrap_or("light");

            let parse_result = parser::parse(code);
            let (_validation, resolved) =
                validator::validate(&parse_result.document, &parse_result.diagnostics);
            let layout_result = layout::compute_layout(&resolved);
            let theme = themes::get_theme(theme_name);
            let svg_output = svg::render_svg(&layout_result, &theme);

            Ok(serde_json::json!({
                "svg": svg_output,
                "width": layout_result.width,
                "height": layout_result.height,
            }))
        }
        "arc_format" => {
            let code = args
                .get("code")
                .and_then(|v| v.as_str())
                .ok_or("Missing 'code' parameter")?;
            let parse_result = parser::parse(code);
            let formatted = fmt::format_document(&parse_result.document);
            Ok(serde_json::json!({ "formatted": formatted }))
        }
        "arc_grammar" => Ok(serde_json::json!({ "grammar": GRAMMAR_SPEC })),
        _ => Err(format!("Unknown tool: {}", name)),
    }
}

// ── MCP Server Loop ─────────────────────────────────────────────

pub fn run_mcp_server() -> io::Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let reader = stdin.lock();

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let request: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                let response = JsonRpcResponse {
                    jsonrpc: "2.0".into(),
                    id: None,
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32700,
                        message: format!("Parse error: {}", e),
                    }),
                };
                let json = serde_json::to_string(&response).unwrap();
                writeln!(stdout, "{}", json)?;
                stdout.flush()?;
                continue;
            }
        };

        let response = match request.method.as_str() {
            "initialize" => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: request.id,
                result: Some(serde_json::json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "arc",
                        "version": env!("CARGO_PKG_VERSION"),
                    }
                })),
                error: None,
            },
            "notifications/initialized" => continue,
            "tools/list" => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: request.id,
                result: Some(tools_list()),
                error: None,
            },
            "tools/call" => {
                let tool_name = request
                    .params
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let arguments = request
                    .params
                    .get("arguments")
                    .cloned()
                    .unwrap_or(serde_json::json!({}));

                match execute_tool(tool_name, &arguments) {
                    Ok(result) => JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id: request.id,
                        result: Some(serde_json::json!({
                            "content": [{
                                "type": "text",
                                "text": serde_json::to_string_pretty(&result).unwrap()
                            }]
                        })),
                        error: None,
                    },
                    Err(e) => JsonRpcResponse {
                        jsonrpc: "2.0".into(),
                        id: request.id,
                        result: Some(serde_json::json!({
                            "content": [{
                                "type": "text",
                                "text": format!("Error: {}", e)
                            }],
                            "isError": true
                        })),
                        error: None,
                    },
                }
            }
            _ => JsonRpcResponse {
                jsonrpc: "2.0".into(),
                id: request.id,
                result: None,
                error: Some(JsonRpcError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                }),
            },
        };

        let json = serde_json::to_string(&response).unwrap();
        writeln!(stdout, "{}", json)?;
        stdout.flush()?;
    }

    Ok(())
}

// ── Grammar spec (embeddable in agent system prompts) ───────────

pub const GRAMMAR_SPEC: &str = r#"ARC v0.1 — Architecture Diagram Language

NODES:
  TYPE ID ["Display Label"] [tag1, tag2]
  TYPE = service | db | cache | queue | gateway | user | store | fn | worker | external

CONNECTIONS:
  ID -> ID [: "label"] [tag]     # solid arrow
  ID --> ID [: "label"] [tag]    # dashed arrow (async)
  ID <-> ID [: "label"] [tag]    # bidirectional
  ID -x ID [: "label"]           # blocked/deprecated

GROUPS:
  group "Label" [tags] {
    NodeID
    NodeID, NodeID          # comma-separated refs
    group "Nested" { ... }  # nested groups
  }

DIRECTIVES:
  @direction down|right
  @theme light|dark|blueprint|mono
  @spacing compact|normal|wide

INCLUDES:
  include "path/to/file.arc"

EXAMPLE:
  @direction right
  @theme light
  service Auth "Auth Service" [Go, JWT]
  service API "Core API" [Node.js]
  db Postgres "Main DB" [v16]
  cache Redis [cluster]
  Auth -> API: "validate token"
  API -> Postgres: "read/write"
  API -> Redis: "cache lookup"
  group "AWS VPC" {
    Auth, API, Postgres, Redis
  }
"#;
