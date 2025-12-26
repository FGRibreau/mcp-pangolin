mod pangolin_client;
mod service;
mod swagger;
mod types;

use anyhow::{Context, Result};
use clap::Parser;
use rmcp::{transport::stdio, ServiceExt};
use std::path::PathBuf;
use tracing::info;
use tracing_subscriber::EnvFilter;

use crate::service::PangolinService;
use crate::swagger::SwaggerSpec;

#[derive(Parser, Debug)]
#[command(
    name = "mcp-pangolin",
    about = "MCP server for Pangolin Integration API",
    long_about = "MCP server that exposes all Pangolin API endpoints as tools.\n\n\
                  The server loads an OpenAPI/Swagger specification and exposes all endpoints as MCP tools.\n\n\
                  Environment variables:\n\
                  - PANGOLIN_API_KEY: API key for authentication (required)\n\
                  - PANGOLIN_BASE_URL: Base URL for the Pangolin API (required)\n\
                  - PANGOLIN_READ_ONLY: Set to 'true' to enable read-only mode (optional)",
    version
)]
struct Args {
    /// Path to the OpenAPI/Swagger JSON specification file
    #[arg(short, long, env = "PANGOLIN_OPENAPI_FILE")]
    openapi: Option<PathBuf>,

    /// Inline OpenAPI/Swagger JSON specification (alternative to --openapi file)
    #[arg(long, env = "PANGOLIN_OPENAPI_JSON")]
    openapi_json: Option<String>,

    /// Pangolin API key for authentication
    #[arg(short = 'k', long, env = "PANGOLIN_API_KEY")]
    api_key: String,

    /// Base URL for the Pangolin API (e.g., https://pangolin.example.com/v1)
    #[arg(short, long, env = "PANGOLIN_BASE_URL")]
    base_url: String,

    /// Enable read-only mode (only GET operations are allowed)
    #[arg(short, long, env = "PANGOLIN_READ_ONLY", default_value = "false")]
    read_only: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to stderr (NEVER stdout for stdio transport!)
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let args = Args::parse();

    info!("Starting MCP Pangolin server");

    // Load the OpenAPI spec
    let spec = if let Some(openapi_path) = &args.openapi {
        info!("Loading OpenAPI spec from file: {:?}", openapi_path);
        SwaggerSpec::from_file(openapi_path.to_str().context("Invalid path")?)
            .context("Failed to load OpenAPI specification from file")?
    } else if let Some(openapi_json) = &args.openapi_json {
        info!("Loading OpenAPI spec from inline JSON");
        SwaggerSpec::from_json(openapi_json)
            .context("Failed to parse inline OpenAPI specification")?
    } else {
        anyhow::bail!(
            "Either --openapi (file path) or --openapi-json (inline JSON) must be provided.\n\n\
             Examples:\n\
             \n\
             1. Load from file:\n\
                mcp-pangolin --openapi pangolin-api.json --api-key YOUR_KEY --base-url https://api.example.com/v1\n\
             \n\
             2. Load from inline JSON:\n\
                mcp-pangolin --openapi-json '{{...}}' --api-key YOUR_KEY --base-url https://api.example.com/v1"
        );
    };

    info!(
        "Loaded OpenAPI spec: {} v{}",
        spec.info.title, spec.info.version
    );

    // Create the MCP service
    let service = PangolinService::new(spec, args.api_key, args.base_url, args.read_only)
        .context("Failed to create Pangolin service")?;

    // Start the stdio transport
    info!("Starting stdio transport...");
    let server = service
        .serve(stdio())
        .await
        .context("Failed to start MCP server")?;

    // Wait for the server to complete
    server.waiting().await?;

    info!("MCP server stopped");
    Ok(())
}
