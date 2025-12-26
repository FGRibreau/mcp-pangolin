//! Tests for PANGOLIN_READ_ONLY mode
//!
//! These tests verify that the read-only mode correctly:
//! 1. Blocks write operations (POST, PUT, DELETE, PATCH)
//! 2. Allows read operations (GET)
//! 3. Properly filters the available tools list
//!
//! Run with: cargo test --test read_only_mode

use std::collections::HashMap;

mod test_helpers {
    use super::*;

    /// Sample OpenAPI spec for testing
    pub fn get_test_swagger_spec() -> &'static str {
        r#"{
            "openapi": "3.0.0",
            "info": {
                "title": "Test Pangolin API",
                "version": "v1"
            },
            "servers": [{"url": "/v1"}],
            "paths": {
                "/orgs": {
                    "get": {
                        "description": "List all organizations",
                        "tags": ["Organization"],
                        "parameters": [],
                        "responses": {}
                    }
                },
                "/org": {
                    "put": {
                        "description": "Create a new organization",
                        "tags": ["Organization"],
                        "parameters": [],
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "name": {"type": "string"}
                                        },
                                        "required": ["name"]
                                    }
                                }
                            }
                        },
                        "responses": {}
                    }
                },
                "/org/{orgId}": {
                    "get": {
                        "description": "Get an organization",
                        "tags": ["Organization"],
                        "parameters": [
                            {
                                "name": "orgId",
                                "in": "path",
                                "required": true,
                                "schema": {"type": "string"}
                            }
                        ],
                        "responses": {}
                    },
                    "post": {
                        "description": "Update an organization",
                        "tags": ["Organization"],
                        "parameters": [
                            {
                                "name": "orgId",
                                "in": "path",
                                "required": true,
                                "schema": {"type": "string"}
                            }
                        ],
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "name": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        },
                        "responses": {}
                    },
                    "delete": {
                        "description": "Delete an organization",
                        "tags": ["Organization"],
                        "parameters": [
                            {
                                "name": "orgId",
                                "in": "path",
                                "required": true,
                                "schema": {"type": "string"}
                            }
                        ],
                        "responses": {}
                    }
                },
                "/site/{siteId}": {
                    "get": {
                        "description": "Get a site",
                        "tags": ["Site"],
                        "parameters": [
                            {
                                "name": "siteId",
                                "in": "path",
                                "required": true,
                                "schema": {"type": "number"}
                            }
                        ],
                        "responses": {}
                    },
                    "delete": {
                        "description": "Delete a site",
                        "tags": ["Site"],
                        "parameters": [
                            {
                                "name": "siteId",
                                "in": "path",
                                "required": true,
                                "schema": {"type": "string"}
                            }
                        ],
                        "responses": {}
                    },
                    "post": {
                        "description": "Update a site",
                        "tags": ["Site"],
                        "parameters": [
                            {
                                "name": "siteId",
                                "in": "path",
                                "required": true,
                                "schema": {"type": "string"}
                            }
                        ],
                        "requestBody": {
                            "content": {
                                "application/json": {
                                    "schema": {
                                        "type": "object",
                                        "properties": {
                                            "name": {"type": "string"}
                                        }
                                    }
                                }
                            }
                        },
                        "responses": {}
                    }
                }
            }
        }"#
    }
}

// Import the modules under test
#[path = "../src/types.rs"]
mod types;

#[path = "../src/swagger.rs"]
mod swagger;

use swagger::SwaggerSpec;
use types::HttpMethod;

#[test]
fn test_http_method_is_write_operation() {
    // GET is not a write operation
    assert!(!HttpMethod::Get.is_write_operation());

    // POST, PUT, DELETE, PATCH are write operations
    assert!(HttpMethod::Post.is_write_operation());
    assert!(HttpMethod::Put.is_write_operation());
    assert!(HttpMethod::Delete.is_write_operation());
    assert!(HttpMethod::Patch.is_write_operation());
}

#[test]
fn test_swagger_spec_parsing() {
    let spec = SwaggerSpec::from_json(test_helpers::get_test_swagger_spec())
        .expect("Failed to parse test swagger spec");

    assert_eq!(spec.info.title, "Test Pangolin API");
    assert_eq!(spec.info.version, "v1");
}

#[test]
fn test_extract_endpoints_from_spec() {
    let spec = SwaggerSpec::from_json(test_helpers::get_test_swagger_spec())
        .expect("Failed to parse test swagger spec");

    let endpoints = spec.extract_endpoints();

    // Should have 8 endpoints total:
    // GET /orgs, PUT /org, GET /org/{orgId}, POST /org/{orgId}, DELETE /org/{orgId},
    // GET /site/{siteId}, DELETE /site/{siteId}, POST /site/{siteId}
    assert_eq!(endpoints.len(), 8);

    // Count by method
    let get_count = endpoints
        .iter()
        .filter(|e| e.method == HttpMethod::Get)
        .count();
    let post_count = endpoints
        .iter()
        .filter(|e| e.method == HttpMethod::Post)
        .count();
    let put_count = endpoints
        .iter()
        .filter(|e| e.method == HttpMethod::Put)
        .count();
    let delete_count = endpoints
        .iter()
        .filter(|e| e.method == HttpMethod::Delete)
        .count();

    assert_eq!(get_count, 3, "Expected 3 GET endpoints");
    assert_eq!(post_count, 2, "Expected 2 POST endpoints");
    assert_eq!(put_count, 1, "Expected 1 PUT endpoint");
    assert_eq!(delete_count, 2, "Expected 2 DELETE endpoints");
}

#[test]
fn test_read_only_mode_filters_write_endpoints() {
    let spec = SwaggerSpec::from_json(test_helpers::get_test_swagger_spec())
        .expect("Failed to parse test swagger spec");

    let endpoints = spec.extract_endpoints();

    // Simulate read-only mode filtering
    let read_only_endpoints: Vec<_> = endpoints
        .iter()
        .filter(|e| !e.method.is_write_operation())
        .collect();

    // Should only have GET endpoints (3 total)
    assert_eq!(read_only_endpoints.len(), 3);

    // All remaining endpoints should be GET
    for endpoint in &read_only_endpoints {
        assert_eq!(
            endpoint.method,
            HttpMethod::Get,
            "Read-only mode should only include GET endpoints"
        );
    }
}

#[test]
fn test_full_mode_includes_all_endpoints() {
    let spec = SwaggerSpec::from_json(test_helpers::get_test_swagger_spec())
        .expect("Failed to parse test swagger spec");

    let endpoints = spec.extract_endpoints();

    // Simulate full mode (no filtering)
    let full_mode_endpoints: Vec<_> = endpoints.iter().collect();

    // Should have all 8 endpoints
    assert_eq!(full_mode_endpoints.len(), 8);
}

#[test]
fn test_endpoint_names_are_generated_correctly() {
    let spec = SwaggerSpec::from_json(test_helpers::get_test_swagger_spec())
        .expect("Failed to parse test swagger spec");

    let endpoints = spec.extract_endpoints();
    let names: Vec<_> = endpoints.iter().map(|e| e.name.as_str()).collect();

    // Check that expected tool names are present
    assert!(
        names.contains(&"orgs"),
        "Should have 'orgs' endpoint for GET /orgs"
    );
    assert!(
        names.contains(&"create_org"),
        "Should have 'create_org' endpoint for PUT /org"
    );
    assert!(
        names.contains(&"org_by_orgId"),
        "Should have 'org_by_orgId' endpoint for GET /org/{{orgId}}"
    );
    assert!(
        names.contains(&"update_org_by_orgId"),
        "Should have 'update_org_by_orgId' endpoint for POST /org/{{orgId}}"
    );
    assert!(
        names.contains(&"delete_org_by_orgId"),
        "Should have 'delete_org_by_orgId' endpoint for DELETE /org/{{orgId}}"
    );
}

#[test]
fn test_path_parameters_are_extracted() {
    let spec = SwaggerSpec::from_json(test_helpers::get_test_swagger_spec())
        .expect("Failed to parse test swagger spec");

    let endpoints = spec.extract_endpoints();

    // Find the GET /org/{orgId} endpoint
    let org_endpoint = endpoints
        .iter()
        .find(|e| e.name == "org_by_orgId" && e.method == HttpMethod::Get)
        .expect("Should find org_by_orgId GET endpoint");

    // Should have orgId as a path parameter
    assert_eq!(org_endpoint.path_params.len(), 1);
    assert_eq!(org_endpoint.path_params[0].name, "orgId");
    assert!(org_endpoint.path_params[0].required);
}

#[test]
fn test_request_body_is_extracted() {
    let spec = SwaggerSpec::from_json(test_helpers::get_test_swagger_spec())
        .expect("Failed to parse test swagger spec");

    let endpoints = spec.extract_endpoints();

    // Find the PUT /org endpoint (create organization)
    let create_org_endpoint = endpoints
        .iter()
        .find(|e| e.name == "create_org")
        .expect("Should find create_org endpoint");

    // Should have a request body
    assert!(
        create_org_endpoint.request_body.is_some(),
        "PUT /org should have a request body"
    );

    let body = create_org_endpoint.request_body.as_ref().unwrap();
    assert!(
        body.properties.contains_key("name"),
        "Request body should have 'name' property"
    );
    assert!(
        body.required.contains(&"name".to_string()),
        "'name' should be required"
    );
}

#[test]
fn test_tags_are_preserved() {
    let spec = SwaggerSpec::from_json(test_helpers::get_test_swagger_spec())
        .expect("Failed to parse test swagger spec");

    let endpoints = spec.extract_endpoints();

    // Find an organization endpoint
    let org_endpoint = endpoints
        .iter()
        .find(|e| e.name == "orgs")
        .expect("Should find orgs endpoint");

    assert!(
        org_endpoint.tags.contains(&"Organization".to_string()),
        "Endpoint should have Organization tag"
    );

    // Find a site endpoint
    let site_endpoint = endpoints
        .iter()
        .find(|e| e.name == "site_by_siteId" && e.method == HttpMethod::Get)
        .expect("Should find site_by_siteId GET endpoint");

    assert!(
        site_endpoint.tags.contains(&"Site".to_string()),
        "Endpoint should have Site tag"
    );
}

#[test]
fn test_build_url_with_path_params() {
    let mut params = HashMap::new();
    params.insert("orgId".to_string(), "my-org".to_string());
    params.insert("siteId".to_string(), "123".to_string());

    let url = swagger::build_url(
        "https://api.pangolin.example.com/v1",
        "/org/{orgId}/site/{siteId}",
        &params,
    );

    assert_eq!(
        url,
        "https://api.pangolin.example.com/v1/org/my-org/site/123"
    );
}

#[test]
fn test_extract_path_params_from_template() {
    let params = swagger::extract_path_params("/org/{orgId}/site/{siteId}/resource/{resourceId}");

    assert_eq!(params.len(), 3);
    assert!(params.contains(&"orgId".to_string()));
    assert!(params.contains(&"siteId".to_string()));
    assert!(params.contains(&"resourceId".to_string()));
}

#[test]
fn test_write_operation_detection_for_all_methods() {
    // Test all methods explicitly
    let methods = vec![
        (HttpMethod::Get, false),
        (HttpMethod::Post, true),
        (HttpMethod::Put, true),
        (HttpMethod::Delete, true),
        (HttpMethod::Patch, true),
    ];

    for (method, expected_is_write) in methods {
        assert_eq!(
            method.is_write_operation(),
            expected_is_write,
            "{:?} should return {} for is_write_operation()",
            method,
            expected_is_write
        );
    }
}

#[test]
fn test_parameter_type_conversion() {
    use types::ParameterType;

    assert_eq!(
        ParameterType::from_openapi_type("string"),
        ParameterType::String
    );
    assert_eq!(
        ParameterType::from_openapi_type("integer"),
        ParameterType::Integer
    );
    assert_eq!(
        ParameterType::from_openapi_type("number"),
        ParameterType::Number
    );
    assert_eq!(
        ParameterType::from_openapi_type("boolean"),
        ParameterType::Boolean
    );
    assert_eq!(
        ParameterType::from_openapi_type("array"),
        ParameterType::Array
    );
    assert_eq!(
        ParameterType::from_openapi_type("object"),
        ParameterType::Object
    );
    // Unknown types default to string
    assert_eq!(
        ParameterType::from_openapi_type("unknown"),
        ParameterType::String
    );
}

#[test]
fn test_json_schema_type_output() {
    use types::ParameterType;

    assert_eq!(ParameterType::String.to_json_schema_type(), "string");
    assert_eq!(ParameterType::Integer.to_json_schema_type(), "integer");
    assert_eq!(ParameterType::Number.to_json_schema_type(), "number");
    assert_eq!(ParameterType::Boolean.to_json_schema_type(), "boolean");
    assert_eq!(ParameterType::Array.to_json_schema_type(), "array");
    assert_eq!(ParameterType::Object.to_json_schema_type(), "object");
}
