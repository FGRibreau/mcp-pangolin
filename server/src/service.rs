//! MCP Service for Pangolin Integration API

use crate::pangolin_client::PangolinClient;
use crate::swagger::SwaggerSpec;
use crate::types::PangolinEndpoint;
use rmcp::handler::server::ServerHandler;
use rmcp::model::*;
use rmcp::service::{RequestContext, RoleServer};
use rmcp::ErrorData;
use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// MCP Service for Pangolin Integration API
#[derive(Clone)]
pub struct PangolinService {
    /// Pangolin HTTP client
    client: Arc<PangolinClient>,
    /// Available endpoints parsed from Swagger spec
    endpoints: Arc<Vec<PangolinEndpoint>>,
    /// Read-only mode flag
    read_only: bool,
    /// Server info
    api_version: String,
    base_url: String,
}

impl PangolinService {
    /// Create a new PangolinService from Swagger spec
    pub fn new(
        spec: SwaggerSpec,
        api_key: String,
        base_url: String,
        read_only: bool,
    ) -> anyhow::Result<Self> {
        let client = PangolinClient::new(&base_url, api_key)?;
        let endpoints = spec.extract_endpoints();

        let available_count = if read_only {
            endpoints
                .iter()
                .filter(|e| !e.method.is_write_operation())
                .count()
        } else {
            endpoints.len()
        };

        info!(
            "Loaded {} endpoints from Swagger spec ({} available in current mode)",
            endpoints.len(),
            available_count
        );

        if read_only {
            info!("Running in READ-ONLY mode - write operations are disabled");
        }

        Ok(Self {
            client: Arc::new(client),
            endpoints: Arc::new(endpoints),
            read_only,
            api_version: spec.info.version.clone(),
            base_url,
        })
    }

    /// Check if read-only mode is enabled
    pub fn is_read_only(&self) -> bool {
        self.read_only
    }

    /// Get available endpoints (filtered by read-only mode if enabled)
    pub fn get_available_endpoints(&self) -> Vec<&PangolinEndpoint> {
        if self.read_only {
            self.endpoints
                .iter()
                .filter(|e| !e.method.is_write_operation())
                .collect()
        } else {
            self.endpoints.iter().collect()
        }
    }

    /// Find an endpoint by name
    fn find_endpoint(&self, name: &str) -> Option<&PangolinEndpoint> {
        self.endpoints.iter().find(|e| e.name == name)
    }

    /// Convert PangolinEndpoint to MCP Tool definition
    fn endpoint_to_mcp(&self, endpoint: &PangolinEndpoint) -> Tool {
        let mut properties = serde_json::Map::new();
        let mut required = Vec::new();

        // Add path parameters
        for param in &endpoint.path_params {
            let mut prop = serde_json::Map::new();
            prop.insert(
                "type".to_string(),
                serde_json::Value::String(param.param_type.to_json_schema_type().to_string()),
            );
            if let Some(ref desc) = param.description {
                prop.insert(
                    "description".to_string(),
                    serde_json::Value::String(desc.clone()),
                );
            }
            if let Some(ref default) = param.default_value {
                prop.insert("default".to_string(), default.clone());
            }
            properties.insert(param.name.clone(), serde_json::Value::Object(prop));
            if param.required {
                required.push(param.name.clone());
            }
        }

        // Add query parameters
        for param in &endpoint.query_params {
            let mut prop = serde_json::Map::new();
            prop.insert(
                "type".to_string(),
                serde_json::Value::String(param.param_type.to_json_schema_type().to_string()),
            );
            if let Some(ref desc) = param.description {
                prop.insert(
                    "description".to_string(),
                    serde_json::Value::String(desc.clone()),
                );
            }
            if let Some(ref default) = param.default_value {
                prop.insert("default".to_string(), default.clone());
            }
            properties.insert(param.name.clone(), serde_json::Value::Object(prop));
            if param.required {
                required.push(param.name.clone());
            }
        }

        // Add request body properties
        if let Some(ref body) = endpoint.request_body {
            for (name, prop) in &body.properties {
                let mut schema_prop = serde_json::Map::new();
                schema_prop.insert(
                    "type".to_string(),
                    serde_json::Value::String(prop.param_type.to_json_schema_type().to_string()),
                );
                if let Some(ref desc) = prop.description {
                    schema_prop.insert(
                        "description".to_string(),
                        serde_json::Value::String(desc.clone()),
                    );
                }
                if let Some(ref default) = prop.default_value {
                    schema_prop.insert("default".to_string(), default.clone());
                }
                if let Some(ref enum_vals) = prop.enum_values {
                    let enum_arr: Vec<serde_json::Value> = enum_vals
                        .iter()
                        .map(|s| serde_json::Value::String(s.clone()))
                        .collect();
                    schema_prop.insert("enum".to_string(), serde_json::Value::Array(enum_arr));
                }
                properties.insert(name.clone(), serde_json::Value::Object(schema_prop));
            }

            // Add required fields from body
            for req_field in &body.required {
                if !required.contains(req_field) {
                    required.push(req_field.clone());
                }
            }
        }

        let mut schema = serde_json::Map::new();
        schema.insert(
            "type".to_string(),
            serde_json::Value::String("object".to_string()),
        );
        schema.insert(
            "properties".to_string(),
            serde_json::Value::Object(properties),
        );

        if !required.is_empty() {
            let required_arr: Vec<serde_json::Value> = required
                .iter()
                .map(|s| serde_json::Value::String(s.clone()))
                .collect();
            schema.insert(
                "required".to_string(),
                serde_json::Value::Array(required_arr),
            );
        }

        // Build description with method and tags
        let mut desc = format!("[{}] {}", endpoint.method.as_str(), endpoint.description);
        if !endpoint.tags.is_empty() {
            desc.push_str(&format!(" (Tags: {})", endpoint.tags.join(", ")));
        }

        Tool {
            name: Cow::Owned(endpoint.name.clone()),
            description: Some(Cow::Owned(desc)),
            input_schema: Arc::new(schema),
            annotations: None,
            icons: None,
            meta: None,
            output_schema: None,
            title: None,
        }
    }
}

impl ServerHandler for PangolinService {
    fn get_info(&self) -> ServerInfo {
        let mode = if self.read_only {
            "read-only"
        } else {
            "read-write"
        };

        ServerInfo {
            protocol_version: ProtocolVersion::V_2024_11_05,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "mcp-pangolin".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
                icons: None,
                title: None,
                website_url: None,
            },
            instructions: Some(format!(
                "Pangolin Integration API server.\n\
                 Connected to: {}\n\
                 API version: {}\n\
                 Mode: {}\n\
                 Available tools: {}\n\n\
                 Use these tools to manage your Pangolin resources including organizations, sites, resources, roles, users, and more.",
                self.base_url,
                self.api_version,
                mode,
                self.get_available_endpoints().len()
            )),
        }
    }

    async fn list_tools(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<RoleServer>,
    ) -> Result<ListToolsResult, ErrorData> {
        let available = self.get_available_endpoints();
        debug!("Listing {} tools", available.len());

        let tools: Vec<Tool> = available.iter().map(|e| self.endpoint_to_mcp(e)).collect();

        Ok(ListToolsResult {
            tools,
            next_cursor: None,
            meta: None,
        })
    }

    async fn call_tool(
        &self,
        request: CallToolRequestParam,
        _context: RequestContext<RoleServer>,
    ) -> Result<CallToolResult, ErrorData> {
        let tool_name = request.name.as_ref();
        debug!("Calling tool: {}", tool_name);

        // Find the endpoint
        let endpoint = self.find_endpoint(tool_name).ok_or_else(|| {
            ErrorData::invalid_params(format!("Unknown tool: {}", tool_name), None)
        })?;

        // Check read-only mode for write operations
        if self.read_only && endpoint.method.is_write_operation() {
            warn!(
                "Blocked write operation in read-only mode: {} {}",
                endpoint.method.as_str(),
                endpoint.path
            );
            return Ok(CallToolResult {
                content: vec![Content::text(format!(
                    "Error: Write operation '{}' is not allowed in read-only mode. \
                     The server is configured with PANGOLIN_READ_ONLY=true.",
                    tool_name
                ))],
                is_error: Some(true),
                meta: None,
                structured_content: None,
            });
        }

        // Extract parameters from arguments
        let args: HashMap<String, serde_json::Value> = match request.arguments {
            Some(map) => map.into_iter().collect(),
            None => HashMap::new(),
        };

        // Separate path params, query params, and body params
        let mut path_params: HashMap<String, String> = HashMap::new();
        let mut query_params: HashMap<String, String> = HashMap::new();
        let mut body_params: serde_json::Map<String, serde_json::Value> = serde_json::Map::new();

        // Extract path parameters
        for param in &endpoint.path_params {
            if let Some(value) = args.get(&param.name) {
                path_params.insert(param.name.clone(), value_to_string(value));
            } else if param.required {
                return Err(ErrorData::invalid_params(
                    format!("Missing required path parameter: {}", param.name),
                    None,
                ));
            }
        }

        // Extract query parameters
        for param in &endpoint.query_params {
            if let Some(value) = args.get(&param.name) {
                query_params.insert(param.name.clone(), value_to_string(value));
            }
        }

        // Extract body parameters (everything else goes to body)
        if endpoint.request_body.is_some() {
            for (key, value) in &args {
                let is_path_param = endpoint.path_params.iter().any(|p| &p.name == key);
                let is_query_param = endpoint.query_params.iter().any(|p| &p.name == key);

                if !is_path_param && !is_query_param {
                    body_params.insert(key.clone(), value.clone());
                }
            }
        }

        let body = if body_params.is_empty() {
            None
        } else {
            Some(serde_json::Value::Object(body_params))
        };

        // Call the Pangolin API
        match self
            .client
            .call(
                endpoint.method,
                &endpoint.path,
                path_params,
                query_params,
                body,
            )
            .await
        {
            Ok(result) => {
                let text =
                    serde_json::to_string_pretty(&result).unwrap_or_else(|_| result.to_string());

                Ok(CallToolResult {
                    content: vec![Content::text(text)],
                    is_error: Some(false),
                    meta: None,
                    structured_content: None,
                })
            }
            Err(e) => Ok(CallToolResult {
                content: vec![Content::text(format!("Error: {}", e))],
                is_error: Some(true),
                meta: None,
                structured_content: None,
            }),
        }
    }
}

/// Convert a JSON value to a string for URL parameters
fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    }
}
