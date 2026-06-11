use axum::{routing::get, Router};

async fn hello_world() -> &'static str {
    "hello, world"
}

pub async fn main() {

    let app = Router::new().route("/", get(hello_world)); 

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}