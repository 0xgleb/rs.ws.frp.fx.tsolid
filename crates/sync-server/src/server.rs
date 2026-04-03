use crate::handler;
use crate::state::EntityStore;
use axum::routing::get;
use axum::Router;
use socketioxide::SocketIo;
use tower_http::cors::CorsLayer;

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
