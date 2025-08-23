
mod model;
mod services;
mod routes;

use std::sync::Arc;
use std::time::Duration;
use axum::{extract::{ws::{Message, WebSocket}, State, WebSocketUpgrade}, http::HeaderValue, response::IntoResponse, routing::get, Router};
use axum::routing::post;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::sync::broadcast::{self, Sender};
use tokio::time::sleep;
use tower_http::cors::{Any, CorsLayer};
use services::llmdb::LLMDB;
use services::db::DB;
use crate::routes::links::{add_link, delete_link, edit_link, search_links};
use crate::services::p2p;
use crate::services::p2p::P2PClient;

// assert checkers
fn assert_send_sync<T: Send + Sync>() {}

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<DB>,
    pub llmdb: Arc<LLMDB>,
    pub queue: Option<Arc<redis::Client>>,
    pub tx: Sender<String>,
}

impl AppState {
    pub(crate) async fn test() -> Self {
        assert_send_sync::<LLMDB>();
        assert_send_sync::<DB>();


        let mongodb_uri = "mongodb://root:example@localhost:27017";
        let database_name = "db";
        let collection_name = "links";
        let url_llmdb = "http://0.0.0.0:7700";

        let db = Arc::new(
            DB::init(mongodb_uri.to_string(), database_name.to_string(), collection_name.to_string()).await.expect("db init failed"));
        let llmdb: Arc<LLMDB> = Arc::new(LLMDB::init(url_llmdb.to_string()).await.expect("llmdb init failed"));

        //websocket communication
        let (tx, _) = broadcast::channel(100);
        
        // redis connection
        
        let state = AppState{db, llmdb, tx, queue: None};
        state
    }
}

#[tokio::main]
async fn main() {
    assert_send_sync::<LLMDB>();
    assert_send_sync::<DB>();
    dotenv::dotenv().ok();
    /*  set up p2p client*/


    /* db */
    let mongodb_uri = std::env::var("URL_DB").unwrap_or("mongodb://root:example@localhost:27017".to_string());
    let database_name =
        std::env::var("DATABASE_NAME").unwrap_or("db".to_string());
    let collection_name = std::env::var("COLLECTION_NAME").unwrap_or("links".to_string());

    /* llmdb */
    let url_llmdb = std::env::var("URL_LLMDB").unwrap_or("http://0.0.0.0:7700".to_string());
    
    /* queue redis */
    let url_queue = std::env::var("URL_QUEUE").unwrap_or("redis://0.0.0.0:6379/0".to_string());
    let redis_client = Arc::new(redis::Client::open(url_queue).expect("redis connect failed"));
    
    let db = Arc::new(DB::init(mongodb_uri, database_name, collection_name).await.expect("db init failed"));
    let llmdb: Arc<LLMDB> = Arc::new(LLMDB::init(url_llmdb).await.expect("llmdb init failed"));

    //websocket communication
    let (tx, _) = broadcast::channel(100);
    let state = AppState{db, llmdb, tx, queue: Some(redis_client)};


    let app = app(state);
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();

    axum::serve(listener, app).await.unwrap();
}


fn app(state: AppState) -> Router {
    let url_address = std::env::var("URL_FRONTEND").unwrap_or("http://localhost:8080".to_string());
    let cors_layer = CorsLayer::new().allow_methods(Any)
        .allow_headers(Any)
        .allow_origin(vec![url_address.parse().unwrap(), "http://localhost:8080".parse().unwrap(), "http://127.0.0.1:8080".parse().unwrap()]);


    
    //let (tx, mut rx) = mpsc::channel(32);
    //let tx2 = tx.clone();
    

    
    Router::new()
        .route("/", get(|| async {"Home"}))
        .route("/link", post(add_link).put(edit_link).delete(delete_link))
        .route("/search", get(search_links))
        .route("/ws", get(chat_handler))
        .with_state(state)
        .layer(cors_layer)
}

async fn chat_handler(State(state): State<AppState>, ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(|websocket| handle_websocket(state.tx, websocket))
}

async fn handle_websocket(send_to_app: Sender<String>, websocket: WebSocket) {
    let (mut ws_sender,mut ws_receiver) = websocket.split();

    // background tasks to send info the websocket info
    let connection_data = p2p::download_server_params_from_address().await;
    let server_peer_id = connection_data.server_id.as_str();
    let ind = connection_data.server_address.len() - 2;
    let server_address: &str = connection_data.server_address.get(ind).expect("server address not found");

    println!("Server peer id: {}", server_peer_id);
    println!("Server address: {}", server_address);

    let client = P2PClient::new(server_peer_id, server_address).expect("Client could not be created");
    client.start().await;
    // If P2PClient methods take &self and are internally Sync:
    let client = Arc::new(client);

    {
        let client = client.clone();
        tokio::spawn(async move {
            loop {
                // info for validation
                let items = client.get_runtime_content_to_validate().await;
                let my_topics = client.get_my_topics().await;
                let all_content = client.all_content();
                let all_status_votes = client.get_status_voteses();

                if !items.is_empty() {
                    for (key, topic, content, duration) in items {
                        println!("new content: {key} | {topic} | {content:?} | {:?}", duration);

                        let jsoned_message = json!({"key" : key, "topic" : topic, "content": content});
                        ws_sender.send(Message::from(
                            serde_json::to_string(&jsoned_message).expect("It could not parse the json message"))).await.expect("It could not send a message");
                    }
                }
                sleep(Duration::from_secs(1)).await;
            }
        });
    }
    {
        let client = client.clone();
        // logic when we receive messages from websocket
        let mut rx_from_app = send_to_app.subscribe();
        tokio::spawn(async move {
            while let Ok(msg) = rx_from_app.recv().await {
                println!("<- {:?}", msg);
                //get key for status?
                //client.get_status_votes(key);
                //TODO can be blocked
                let votes = client.get_my_topics().await;
                println!("votes={:?}", votes);
                //we want to replay the thread
                //   ws_sender.send(Message::from(msg)).await.unwrap()
            }
        });
    }


    // we receive from the websocket then we send to the channel in the app
    while let Some(msg) = ws_receiver.next().await  {
        if let Ok(msg)  = msg {
            match msg {
                Message::Text(content) => {
                    println!("->{}", content);
                    //we send to another threead from the websocket for avoiding blockers
                    send_to_app.send(content.to_string()).unwrap();
                },
                _ => ()
            }
        }
    }
}
#[cfg(test)]
mod tests {

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use serde_json::json;
    use tower::ServiceExt; // for `app.oneshot()`
    use http_body_util::BodyExt;


    use crate::{app, model::Link, AppState};


    #[tokio::test]
    async fn test_add_link() {
        let state = AppState::test().await;
        let app = app(state);

        let link = Link {
            url: "https://example.com".to_string(),
            tags: vec!["tag1".to_string()],
            description: None
        };

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/link")
                    .header("Content-Type", "application/json")
                    .body(Body::from(serde_json::to_string(&link).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);

        let body = response.into_body();
        let collected = body.collect().await.unwrap();
        let bytes = collected.to_bytes();
        let new_id: String = serde_json::from_slice(&*bytes).unwrap();
        assert!(!new_id.is_empty());
    }

    #[tokio::test]
    async fn test_search_links() {
        let state = AppState::test().await;
        let app = app(state);

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/search?query=example")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body();
        let collected = body.collect().await.unwrap();
        let bytes = collected.to_bytes();
        let links: Vec<Link>= serde_json::from_slice(&*bytes).unwrap();
        assert_eq!(links.len(), 1);
    }

    #[tokio::test]
    async fn test_delete_link() {
        let state = AppState::test().await;
        let app = app(state);

        let id = "fake_id_123";

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/link")
                    .header("Content-Type", "application/json")
                    .body(Body::from(json!(id).to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            response.status() == StatusCode::NO_CONTENT
                || response.status() == StatusCode::INTERNAL_SERVER_ERROR
        );
    }

    #[tokio::test]
    async fn test_edit_link() {
        let state = AppState::test().await;
        let app = app(state);

        let llmdb_link = json!({
        "id": "fake_id_123",
        "link": {
            "url": "https://example.com",
            "tags": ["tag1"],
            "description": "Updated desc"
        }
    });

        let response = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/link")
                    .header("Content-Type", "application/json")
                    .body(Body::from(llmdb_link.to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert!(
            response.status() == StatusCode::OK
                || response.status() == StatusCode::NOT_FOUND
        );
    }

}