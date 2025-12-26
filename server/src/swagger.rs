//! Swagger/OpenAPI specification parser for Pangolin API
//!
//! This module parses the Pangolin OpenAPI 3.0 specification and extracts
//! all endpoints as MCP tools.

use anyhow::{Context, Result};
use indexmap::IndexMap;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::types::{
    EndpointParameter, HttpMethod, PangolinEndpoint, ParameterType, PropertySchema,
    RequestBodySchema,
};

/// Root OpenAPI specification structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwaggerSpec {
    pub openapi: String,
    pub info: SwaggerInfo,
    #[serde(default)]
    pub servers: Vec<SwaggerServer>,
    pub paths: IndexMap<String, PathItem>,
    #[serde(default)]
    pub components: Option<Components>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwaggerInfo {
    pub title: String,
    pub version: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwaggerServer {
    pub url: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PathItem {
    #[serde(default)]
    pub get: Option<Operation>,
    #[serde(default)]
    pub post: Option<Operation>,
    #[serde(default)]
    pub put: Option<Operation>,
    #[serde(default)]
    pub delete: Option<Operation>,
    #[serde(default)]
    pub patch: Option<Operation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub parameters: Vec<Parameter>,
    #[serde(default)]
    pub request_body: Option<RequestBody>,
    #[serde(default)]
    pub security: Vec<serde_json::Value>,
    #[serde(default)]
    pub responses: IndexMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub location: String,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub schema: Option<ParameterSchema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterSchema {
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default, rename = "enum")]
    pub enum_values: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub content: HashMap<String, MediaType>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    #[serde(default)]
    pub schema: Option<Schema>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    #[serde(default)]
    pub properties: Option<HashMap<String, SchemaProperty>>,
    #[serde(default)]
    pub required: Option<Vec<String>>,
    #[serde(default)]
    pub additional_properties: Option<bool>,
    #[serde(default)]
    pub items: Option<Box<SchemaProperty>>,
    #[serde(default, rename = "allOf")]
    pub all_of: Option<Vec<Schema>>,
    #[serde(default, rename = "anyOf")]
    pub any_of: Option<Vec<Schema>>,
    #[serde(default, rename = "oneOf")]
    pub one_of: Option<Vec<Schema>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SchemaProperty {
    #[serde(rename = "type")]
    pub schema_type: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub format: Option<String>,
    #[serde(default)]
    pub default: Option<serde_json::Value>,
    #[serde(default, rename = "enum")]
    pub enum_values: Option<Vec<String>>,
    #[serde(default)]
    pub nullable: Option<bool>,
    #[serde(default)]
    pub min_length: Option<i64>,
    #[serde(default)]
    pub max_length: Option<i64>,
    #[serde(default)]
    pub minimum: Option<f64>,
    #[serde(default)]
    pub maximum: Option<f64>,
    #[serde(default)]
    pub exclusive_minimum: Option<bool>,
    #[serde(default)]
    pub pattern: Option<String>,
    #[serde(default)]
    pub items: Option<Box<SchemaProperty>>,
    #[serde(default)]
    pub properties: Option<HashMap<String, SchemaProperty>>,
    #[serde(default)]
    pub required: Option<Vec<String>>,
    #[serde(default, rename = "anyOf")]
    pub any_of: Option<Vec<SchemaProperty>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Components {
    #[serde(default)]
    pub schemas: Option<HashMap<String, serde_json::Value>>,
    #[serde(default, rename = "securitySchemes")]
    pub security_schemes: Option<HashMap<String, serde_json::Value>>,
    #[serde(default)]
    pub parameters: Option<HashMap<String, serde_json::Value>>,
}

/// Wrapper for the full Swagger document with swaggerDoc field
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SwaggerDocument {
    pub swagger_doc: SwaggerSpec,
    #[serde(default)]
    pub custom_options: Option<serde_json::Value>,
}

impl SwaggerSpec {
    /// Load from file
    pub fn from_file(path: &str) -> Result<Self> {
        let content = std::fs::read_to_string(path).context("Failed to read swagger file")?;
        Self::from_json(&content)
    }

    /// Parse from JSON string
    pub fn from_json(json: &str) -> Result<Self> {
        // First try to parse as SwaggerDocument (with swaggerDoc wrapper)
        if let Ok(doc) = serde_json::from_str::<SwaggerDocument>(json) {
            return Ok(doc.swagger_doc);
        }
        // Otherwise try direct parsing
        serde_json::from_str(json).context("Failed to parse swagger JSON")
    }

    /// Get the base URL from servers
    pub fn get_base_url(&self) -> Option<String> {
        self.servers.first().map(|s| s.url.clone())
    }

    /// Extract all endpoints from the specification
    pub fn extract_endpoints(&self) -> Vec<PangolinEndpoint> {
        let mut endpoints = Vec::new();

        for (path, path_item) in &self.paths {
            // Process each HTTP method
            if let Some(op) = &path_item.get {
                if let Some(endpoint) = self.create_endpoint(path, HttpMethod::Get, op) {
                    endpoints.push(endpoint);
                }
            }
            if let Some(op) = &path_item.post {
                if let Some(endpoint) = self.create_endpoint(path, HttpMethod::Post, op) {
                    endpoints.push(endpoint);
                }
            }
            if let Some(op) = &path_item.put {
                if let Some(endpoint) = self.create_endpoint(path, HttpMethod::Put, op) {
                    endpoints.push(endpoint);
                }
            }
            if let Some(op) = &path_item.delete {
                if let Some(endpoint) = self.create_endpoint(path, HttpMethod::Delete, op) {
                    endpoints.push(endpoint);
                }
            }
            if let Some(op) = &path_item.patch {
                if let Some(endpoint) = self.create_endpoint(path, HttpMethod::Patch, op) {
                    endpoints.push(endpoint);
                }
            }
        }

        endpoints
    }

    fn create_endpoint(
        &self,
        path: &str,
        method: HttpMethod,
        operation: &Operation,
    ) -> Option<PangolinEndpoint> {
        // Generate tool name from path and method
        let name = generate_tool_name(path, method);

        // Get description
        let description = operation
            .description
            .clone()
            .or_else(|| operation.summary.clone())
            .unwrap_or_else(|| format!("{} {}", method.as_str(), path));

        // Extract path and query parameters
        let mut path_params = Vec::new();
        let mut query_params = Vec::new();

        for param in &operation.parameters {
            let endpoint_param = convert_parameter(param);
            match param.location.as_str() {
                "path" => path_params.push(endpoint_param),
                "query" => query_params.push(endpoint_param),
                _ => {}
            }
        }

        // Extract request body schema
        let request_body = operation
            .request_body
            .as_ref()
            .and_then(|rb| extract_request_body_schema(rb));

        Some(PangolinEndpoint {
            name,
            method,
            path: path.to_string(),
            description,
            tags: operation.tags.clone(),
            path_params,
            query_params,
            request_body,
        })
    }
}

/// Generate a tool name from path and method
fn generate_tool_name(path: &str, method: HttpMethod) -> String {
    // Remove leading slash and replace special chars
    let clean_path = path
        .trim_start_matches('/')
        .replace('/', "_")
        .replace('-', "_");

    // Replace path parameters like {orgId} with their names
    let param_re = Regex::new(r"\{([^}]+)\}").unwrap();
    let name_with_params = param_re.replace_all(&clean_path, "by_$1");

    // Add method prefix for non-GET methods
    let method_prefix = match method {
        HttpMethod::Get => "",
        HttpMethod::Post => "update_",
        HttpMethod::Put => "create_",
        HttpMethod::Delete => "delete_",
        HttpMethod::Patch => "patch_",
    };

    // Handle special case for root path
    if name_with_params.is_empty() {
        return format!("{}health_check", method_prefix);
    }

    format!("{}{}", method_prefix, name_with_params)
}

/// Convert OpenAPI parameter to our EndpointParameter type
fn convert_parameter(param: &Parameter) -> EndpointParameter {
    let schema = param.schema.as_ref();

    let param_type = schema
        .and_then(|s| s.schema_type.as_ref())
        .map(|t| ParameterType::from_openapi_type(t))
        .unwrap_or(ParameterType::String);

    let default_value = schema.and_then(|s| s.default.clone());

    EndpointParameter {
        name: param.name.clone(),
        param_type,
        required: param.required,
        description: param.description.clone(),
        default_value,
    }
}

/// Extract request body schema from OpenAPI request body
fn extract_request_body_schema(request_body: &RequestBody) -> Option<RequestBodySchema> {
    // Get JSON content type
    let media_type = request_body
        .content
        .get("application/json")
        .or_else(|| request_body.content.values().next())?;

    let schema = media_type.schema.as_ref()?;

    // Handle allOf, anyOf, oneOf by merging properties
    let mut all_properties = HashMap::new();
    let mut all_required = Vec::new();

    // Process direct properties
    if let Some(props) = &schema.properties {
        for (name, prop) in props {
            all_properties.insert(name.clone(), convert_schema_property(name, prop));
        }
    }
    if let Some(req) = &schema.required {
        all_required.extend(req.clone());
    }

    // Process allOf
    if let Some(all_of) = &schema.all_of {
        for sub_schema in all_of {
            if let Some(props) = &sub_schema.properties {
                for (name, prop) in props {
                    // props is HashMap<String, SchemaProperty>, so prop is already SchemaProperty
                    all_properties.insert(name.clone(), convert_schema_property(name, prop));
                }
            }
            if let Some(req) = &sub_schema.required {
                all_required.extend(req.clone());
            }
        }
    }

    // Process anyOf (take first one as example)
    if let Some(any_of) = &schema.any_of {
        if let Some(first) = any_of.first() {
            if let Some(props) = &first.properties {
                for (name, prop) in props {
                    // props is HashMap<String, SchemaProperty>, so prop is already SchemaProperty
                    all_properties.insert(name.clone(), convert_schema_property(name, prop));
                }
            }
            if let Some(req) = &first.required {
                all_required.extend(req.clone());
            }
        }
    }

    if all_properties.is_empty() {
        return None;
    }

    Some(RequestBodySchema {
        content_type: "application/json".to_string(),
        properties: all_properties,
        required: all_required,
    })
}


/// Convert OpenAPI SchemaProperty to our PropertySchema type
fn convert_schema_property(name: &str, prop: &SchemaProperty) -> PropertySchema {
    let param_type = prop
        .schema_type
        .as_ref()
        .map(|t| ParameterType::from_openapi_type(t))
        .unwrap_or(ParameterType::String);

    let items = prop.items.as_ref().map(|i| {
        Box::new(convert_schema_property("item", i))
    });

    PropertySchema {
        name: name.to_string(),
        param_type,
        description: prop.description.clone(),
        default_value: prop.default.clone(),
        enum_values: prop.enum_values.clone(),
        nullable: prop.nullable.unwrap_or(false),
        min_length: prop.min_length,
        max_length: prop.max_length,
        minimum: prop.minimum,
        maximum: prop.maximum,
        pattern: prop.pattern.clone(),
        items,
    }
}

/// Extract path parameters from a path template
pub fn extract_path_params(path: &str) -> Vec<String> {
    let re = Regex::new(r"\{([^}]+)\}").unwrap();
    re.captures_iter(path)
        .map(|cap| cap[1].to_string())
        .collect()
}

/// Build the actual URL by substituting path parameters
pub fn build_url(base_url: &str, path: &str, path_params: &HashMap<String, String>) -> String {
    let mut url = format!("{}{}", base_url.trim_end_matches('/'), path);

    for (key, value) in path_params {
        url = url.replace(&format!("{{{}}}", key), value);
    }

    url
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_tool_name() {
        assert_eq!(
            generate_tool_name("/org/{orgId}/site", HttpMethod::Get),
            "org_by_orgId_site"
        );
        assert_eq!(
            generate_tool_name("/org/{orgId}/site", HttpMethod::Put),
            "create_org_by_orgId_site"
        );
        assert_eq!(
            generate_tool_name("/site/{siteId}", HttpMethod::Delete),
            "delete_site_by_siteId"
        );
        assert_eq!(
            generate_tool_name("/", HttpMethod::Get),
            "health_check"
        );
    }

    #[test]
    fn test_extract_path_params() {
        let params = extract_path_params("/org/{orgId}/site/{siteId}/resource/{resourceId}");
        assert_eq!(params, vec!["orgId", "siteId", "resourceId"]);
    }

    #[test]
    fn test_build_url() {
        let mut params = HashMap::new();
        params.insert("orgId".to_string(), "org123".to_string());
        params.insert("siteId".to_string(), "site456".to_string());

        let url = build_url(
            "https://api.pangolin.example.com/v1",
            "/org/{orgId}/site/{siteId}",
            &params,
        );
        assert_eq!(url, "https://api.pangolin.example.com/v1/org/org123/site/site456");
    }
}
