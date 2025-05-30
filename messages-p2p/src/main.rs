use tokio::{select, signal};
use crate::p2p::bootstrap::BootstrapServer;
use std::net::{SocketAddr, IpAddr};
use serde::{Deserialize, Serialize};
use axum::{
    extract::State,
    routing::get,
    Router,
};
use std::sync::Arc;


pub mod p2p;

pub fn init_logging() {
    let _ = env_logger::builder()
        .is_test(false)
        .filter_level(log::LevelFilter::Debug)
        .try_init();
    log::info!("Logging initialized for Server");
}

//HTTP server
struct TrackerServer {
    ip: String,
    port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DataConnection {
    id: String,
    addresses: Vec<String>,
}

#[derive(Clone)]
struct AppState {
    data_connection: DataConnection,
}

async fn get_tracker_info(
    State(state): State<Arc<AppState>>,
) -> String {
    log::info!("Returning tracker info");
    match serde_json::to_string(&state.data_connection.clone()) {
        Ok(serialized) => {
                serialized
        }
        Err(e) => {
            eprintln!("{:?}", e);
            "{\"error\": \"error parsing\"}".to_string()
        }
    }
}


impl TrackerServer {
    async fn new(ip: String, port: u16,) -> Self{
        log::info!("Initializing TrackerServer...");
        TrackerServer{ip, port}
    }

    async fn run(&self, data_connection: (String, Vec<String>)) -> anyhow::Result<()> {
        let data_connection =  DataConnection{  id: data_connection.0, addresses: data_connection.1};
        let shared_state = Arc::new(AppState{data_connection});

        // let addr_str = format!("{}:{}", self.ip, self.port);
        let ip: IpAddr = self.ip.parse()?;
        let addr = SocketAddr::new(ip, self.port);
        log::info!("TrackerServer listening on http://{:?}", addr);

        let shared_state = Arc::clone(&shared_state);

        let app = Router::new()
            .route(
                "/tracker",
                get(get_tracker_info))
            .with_state(shared_state.clone());


        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        axum::serve(listener, app).await.unwrap();

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    // Graceful shutdown control
    init_logging();
    let tracker_address = "0.0.0.0";
    let tracker_port = 3000;
    let tracker = TrackerServer::new(tracker_address.to_string(), tracker_port).await;

    /*  p2p bootstrap server */
    let mut p2p_bootstrap_server = BootstrapServer::new().await.unwrap();
    let data_connection = p2p_bootstrap_server.data_connection();
    select! {
        res = tracker.run(data_connection) => match res {
            Ok(_) => println!("Tracker server exited."),
            Err(e) => eprintln!("Tracker server error: {:?}", e),
        },
        res = p2p_bootstrap_server.run() => match res {
            Ok(_) => println!("libp2p server exited."),
            Err(e) => eprintln!("libp2p server error: {:?}", e),
        },
        _ = signal::ctrl_c() => println!("Received Ctrl+C, shutting down."),
    }
}




