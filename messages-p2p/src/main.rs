use crate::p2p::bootstrap::BootstrapServer;
use axum::{extract::State, routing::get, Router};
use dotenv::dotenv;
use libp2p::identity::Keypair;
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::env;
use std::net::{IpAddr, SocketAddr};
use std::sync::Arc;
use tokio::{select, signal};

pub mod p2p;

static DEFAULT_LISTENS_ON: Lazy<Vec<String>> = Lazy::new(|| {
    vec!["/ip4/127.0.0.1/tcp/15000", "/ip4/0.0.0.0/tcp/15000"]
        .into_iter()
        .map(String::from)
        .collect()
});

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

async fn get_tracker_info(State(state): State<Arc<AppState>>) -> String {
    log::info!("Returning tracker info");
    serde_json::to_string(&state.data_connection.clone()).unwrap_or_else(|e| {
        eprintln!("{e:?}");
        "{\"error\": \"error parsing\"}".to_string()
    })
}

impl TrackerServer {
    async fn new(ip: String, port: u16) -> Self {
        log::info!("Initializing TrackerServer...");
        TrackerServer { ip, port }
    }

    async fn run(&self, data_connection: (String, Vec<String>)) -> anyhow::Result<()> {
        let data_connection = DataConnection {
            id: data_connection.0,
            addresses: data_connection.1,
        };
        let shared_state = Arc::new(AppState { data_connection });

        // let addr_str = format!("{}:{}", self.ip, self.port);
        let ip: IpAddr = self.ip.parse()?;
        let addr = SocketAddr::new(ip, self.port);
        log::info!("TrackerServer listening on http://{addr:?}");

        let shared_state = Arc::clone(&shared_state);

        let app = Router::new()
            .route("/tracker", get(get_tracker_info))
            .with_state(shared_state.clone());

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

#[tokio::main]
async fn main() {
    // Graceful shutdown control
    init_logging();
    dotenv().ok(); // carga el archivo .env

    let tracker_address = env::var("TRACKER_ADDRESS").unwrap_or("0.0.0.0".to_string());
    let tracker_port: u16 = env::var("TRACKER_PORT")
        .unwrap_or("3000".to_string())
        .parse()
        .expect("TRACKER_PORT is not a number");

    let listen_ons = match env::var("LISTEN_URLS") {
        Ok(val) => val
            .split(',')
            .map(|s| s.trim().to_string())
            .collect::<Vec<_>>(),
        Err(_) => DEFAULT_LISTENS_ON.to_vec(),
    };
    let keypair = Keypair::generate_ed25519();
    println!(
        "TRACKER_ADDRESS: host={} port={}",
        tracker_address, tracker_port
    );

    let p2p_port = 15000;
    println!("Listening on = {:?} ", listen_ons);
    println!("p2p_port = {:?}", p2p_port);
    println!("Public key for server = {:?} ", keypair.public());

    let tracker = TrackerServer::new(tracker_address.to_string(), tracker_port).await;

    /*  p2p bootstrap server */
    let mut p2p_bootstrap_server = BootstrapServer::new(keypair, listen_ons, vec![], p2p_port)
        .await
        .unwrap();
    let data_connection = p2p_bootstrap_server.data_connection();
    select! {
        res = tracker.run(data_connection) => match res {
            Ok(_) => println!("Tracker server exited."),
            Err(e) => eprintln!("Tracker server error: {e:?}"),
        },
        res = p2p_bootstrap_server.run() => match res {
            Ok(_) => println!("libp2p server exited."),
            Err(e) => eprintln!("libp2p server error: {e:?}"),
        },
        _ = signal::ctrl_c() => println!("Received Ctrl+C, shutting down."),
    }
}
