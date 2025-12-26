use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// HTTP method type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl HttpMethod {
    /// Returns true if this method is considered a write operation
    pub fn is_write_operation(&self) -> bool {
        matches!(self, HttpMethod::Post | HttpMethod::Put | HttpMethod::Delete | HttpMethod::Patch)
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            HttpMethod::Get => "GET",
            HttpMethod::Post => "POST",
            HttpMethod::Put => "PUT",
            HttpMethod::Delete => "DELETE",
            HttpMethod::Patch => "PATCH",
        }
    }
}

/// Represents a Pangolin API endpoint with its metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PangolinEndpoint {
    /// Tool name (snake_case version of operation)
    pub name: String,
    /// HTTP method
    pub method: HttpMethod,
    /// Path template (e.g., "/org/{orgId}/site")
    pub path: String,
    /// Description of the endpoint
    pub description: String,
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Path parameters
    pub path_params: Vec<EndpointParameter>,
    /// Query parameters
    pub query_params: Vec<EndpointParameter>,
    /// Request body schema (if any)
    pub request_body: Option<RequestBodySchema>,
}

/// Represents a parameter for an endpoint
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EndpointParameter {
    pub name: String,
    pub param_type: ParameterType,
    pub required: bool,
    pub description: Option<String>,
    pub default_value: Option<serde_json::Value>,
}

/// Possible parameter types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParameterType {
    String,
    Integer,
    Number,
    Boolean,
    Array,
    Object,
}

impl ParameterType {
    pub fn to_json_schema_type(&self) -> &'static str {
        match self {
            ParameterType::String => "string",
            ParameterType::Integer => "integer",
            ParameterType::Number => "number",
            ParameterType::Boolean => "boolean",
            ParameterType::Array => "array",
            ParameterType::Object => "object",
        }
    }

    pub fn from_openapi_type(type_str: &str) -> Self {
        match type_str {
            "integer" => ParameterType::Integer,
            "number" => ParameterType::Number,
            "boolean" => ParameterType::Boolean,
            "array" => ParameterType::Array,
            "object" => ParameterType::Object,
            _ => ParameterType::String,
        }
    }
}

/// Request body schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBodySchema {
    pub content_type: String,
    pub properties: HashMap<String, PropertySchema>,
    pub required: Vec<String>,
}

/// Property schema for request body
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PropertySchema {
    pub name: String,
    pub param_type: ParameterType,
    pub description: Option<String>,
    pub default_value: Option<serde_json::Value>,
    pub enum_values: Option<Vec<String>>,
    pub nullable: bool,
    pub min_length: Option<i64>,
    pub max_length: Option<i64>,
    pub minimum: Option<f64>,
    pub maximum: Option<f64>,
    pub pattern: Option<String>,
    pub items: Option<Box<PropertySchema>>,
}
