// SPDX-License-Identifier: PMPL-1.0-or-later
// SPDX-FileCopyrightText: 2026 Jonathan D.A. Jewell

//! TypeLL JSON-RPC Server — exposes the verification kernel to PanLL.
//!
//! Listens on `localhost:7800` (matching `TypeLLCmd.res` in PanLL).
//!
//! # Endpoints
//!
//! | Path              | Method | Handler                |
//! |-------------------|--------|------------------------|
//! | `/api/v1/health`  | GET    | Health check            |
//! | `/api/v1/check`   | POST   | Bidirectional checking  |
//! | `/api/v1/infer`   | POST   | Type inference          |
//! | `/api/v1/refine`  | POST   | Refinement types        |
//! | `/api/v1/compute` | POST   | Type-level computation  |
//! | `/api/v1/signatures` | GET | Signature catalogue     |
//! | `/api/v1/universes`  | GET | Universe hierarchy      |
//!
//! # Usage
//!
//! ```sh
//! cargo run -p typell-server
//! # Server listens on http://localhost:7800
//! # PanLL connects via TypeLLCmd.res
//! ```

#![forbid(unsafe_code)]
mod handlers;

use axum::routing::{get, post};
use axum::Router;
use tower_http::cors::{Any, CorsLayer};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() {
    // Initialise tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("typell_server=info")),
        )
        .init();

    let port: u16 = std::env::var("TYPELL_PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(7800);

    // CORS layer for browser-based PanLL access
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/api/v1/health", get(handlers::health))
        .route("/api/v1/check", post(handlers::check))
        .route("/api/v1/infer", post(handlers::infer))
        .route("/api/v1/refine", post(handlers::refine))
        .route("/api/v1/compute", post(handlers::compute))
        .route("/api/v1/signatures", get(handlers::list_signatures))
        .route("/api/v1/universes", get(handlers::universes))
        // Additional endpoints consumed by PanLL's TypeLLCmd.res
        .route("/api/v1/infer-usage", post(handlers::infer_usage))
        .route("/api/v1/check-effects", post(handlers::check_effects))
        .route("/api/v1/check-dimensional", post(handlers::check_dimensional))
        .route("/api/v1/generate-obligations", post(handlers::generate_obligations))
        // VCL-total endpoint for 10-level type-safe query checking
        .route("/api/v1/vcl-total/check", post(handlers::vcl_ut_check))
        .layer(cors);

    let addr = format!("127.0.0.1:{port}");
    tracing::info!("TypeLL server listening on http://{addr}");

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("failed to bind to address");

    axum::serve(listener, app)
        .await
        .expect("server error");
}
