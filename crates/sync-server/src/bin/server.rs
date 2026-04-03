use sync_server::build_app;
use sync_server::state::EntityStore;

#[tokio::main]
async fn main() {
    let store = EntityStore::new();
    let (app, _io) = build_app(store);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    println!("Server listening on http://0.0.0.0:3000");

    axum::serve(listener, app).await.unwrap();
}
