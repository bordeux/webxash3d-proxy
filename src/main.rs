//! WebRTC to UDP proxy for CS 1.6 / Half-Life game servers.
//!
//! This proxy enables browser clients to connect to traditional game servers
//! by bridging WebRTC data channels to UDP sockets.

mod bridge;
mod config;
mod signaling;

use std::sync::Arc;

use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        State,
    },
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use clap::Parser;
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;
use tracing::info;
use tracing_subscriber::EnvFilter;

use config::Config;

/// Application state shared across handlers
#[derive(Clone)]
struct AppState {
    config: Arc<Config>,
}

/// Client configuration response
#[derive(Serialize)]
struct ClientConfig {
    arguments: Vec<String>,
    console: Vec<String>,
    game_dir: String,
    libraries: ClientLibraries,
    dynamic_libraries: Vec<String>,
    files_map: std::collections::HashMap<String, String>,
}

#[derive(Serialize)]
struct ClientLibraries {
    client: String,
    server: String,
    extras: String,
    menu: String,
    filesystem: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI arguments
    let config = Config::parse();

    // Setup logging
    let filter = if config.verbose {
        EnvFilter::new("debug")
    } else {
        EnvFilter::new("info")
    };

    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_target(false)
        .init();

    info!("Starting webxash3d-proxy");
    info!("Game server: {}", config.server);
    info!("Listen address: {}", config.listen_addr());
    info!("Game directory: {}", config.game_dir);

    if let Some(ref ip) = config.public_ip {
        info!("Public IP for ICE: {}", ip);
    }

    let state = AppState {
        config: Arc::new(config.clone()),
    };

    // Build router
    let mut app = Router::new()
        .route("/ws", get(ws_handler))
        .route("/websocket", get(ws_handler))
        .route("/health", get(health_handler))
        .route("/config", get(config_handler))
        .layer(
            CorsLayer::new()
                .allow_origin(Any)
                .allow_methods(Any)
                .allow_headers(Any),
        )
        .with_state(state);

    // Optionally serve static files
    if let Some(ref static_dir) = config.static_dir {
        info!("Serving static files from: {}", static_dir);
        app =
            app.fallback_service(ServeDir::new(static_dir).append_index_html_on_directories(true));
    }

    // Start server
    let listener = tokio::net::TcpListener::bind(config.listen_addr()).await?;
    info!("Server listening on http://{}", config.listen_addr());

    axum::serve(listener, app).await?;

    Ok(())
}

/// WebSocket upgrade handler
async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    let client_id = uuid::Uuid::new_v4().to_string()[..8].to_string();
    ws.on_upgrade(move |socket| handle_socket(socket, state, client_id))
}

/// Handle upgraded WebSocket connection
async fn handle_socket(socket: WebSocket, state: AppState, client_id: String) {
    signaling::handle_websocket(socket, state.config, client_id).await;
}

/// Health check endpoint
async fn health_handler() -> &'static str {
    "OK"
}

/// Client configuration endpoint
/// Returns configuration needed by the `Xash3D` WASM client
async fn config_handler(State(state): State<AppState>) -> Json<ClientConfig> {
    let game_dir = &state.config.game_dir;

    // Build files_map to translate .so requests to .wasm files
    let mut files_map = std::collections::HashMap::new();

    // Map server library requests (engine requests .so, we serve .wasm)
    files_map.insert(
        "dlls/cs_emscripten_wasm32.so".to_string(),
        format!("/{game_dir}/dlls/cs_emscripten_wasm32.wasm"),
    );
    files_map.insert(
        "dlls/hl_emscripten_wasm32.so".to_string(),
        format!("/{game_dir}/dlls/cs_emscripten_wasm32.wasm"),
    );
    files_map.insert(
        "/rwdir/filesystem_stdio.wasm".to_string(),
        "/filesystem_stdio.wasm".to_string(),
    );

    Json(ClientConfig {
        arguments: vec![
            "-windowed".to_string(),
            "-game".to_string(),
            game_dir.clone(),
        ],
        console: state.config.get_console_commands(),
        game_dir: game_dir.clone(),
        libraries: ClientLibraries {
            // These paths are relative to the static directory
            // The client will load them from these URLs
            client: format!("/{game_dir}/cl_dlls/client_emscripten_wasm32.wasm"),
            server: format!("/{game_dir}/dlls/cs_emscripten_wasm32.wasm"),
            extras: format!("/{game_dir}/extras.pk3"),
            menu: format!("/{game_dir}/cl_dlls/menu_emscripten_wasm32.wasm"),
            filesystem: "/filesystem_stdio.wasm".to_string(),
        },
        dynamic_libraries: vec![
            "dlls/cs_emscripten_wasm32.so".to_string(),
            "/rwdir/filesystem_stdio.wasm".to_string(),
        ],
        files_map,
    })
}
