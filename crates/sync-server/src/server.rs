use crate::handler;
use crate::state::EntityStore;
use axum::routing::get;
use axum::Router;
use socketioxide::SocketIo;
use tower_http::cors::CorsLayer;

/// Build the Axum application with Socket.IO layer.
///
/// Returns a tuple of `(Router, SocketIo)`:
/// - The `Router` is ready to be served via `axum::serve`.
/// - The `SocketIo` handle can be used for programmatic access to
///   connected sockets (e.g., in tests).
///
/// ## What it sets up
///
/// - **Socket.IO namespace `/`** -- All clients connect to the root
///   namespace, join the `"orders"` room, and register `"handshake"` /
///   `"command"` event handlers.
/// - **`GET /health`** -- Returns `"ok"` for load balancer health checks.
/// - **CORS** -- Permissive CORS layer for development. Should be
///   tightened for production deployments.
///
/// ## Example
///
/// ```rust,no_run
/// use sync_server::build_app;
/// use sync_server::state::EntityStore;
///
/// # async fn run() {
/// let store = EntityStore::new();
/// let (app, io) = build_app(store);
///
/// let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
/// axum::serve(listener, app).await.unwrap();
/// # }
/// ```
pub fn build_app(store: EntityStore) -> (Router, SocketIo) {
    let (layer, io) = SocketIo::builder()
        .with_state(store.clone())
        .build_layer();

    io.ns("/", |socket: socketioxide::extract::SocketRef| async move {
        let _ = socket.join("orders");

        socket.on("handshake", handler::on_handshake);
        socket.on("command", handler::on_command);
    });

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .layer(layer)
        .layer(CorsLayer::permissive());

    (app, io)
}
