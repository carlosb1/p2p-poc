use axum::Router;
use axum::routing::get;
use tokio::net::unix::SocketAddr;
use tokio::{select, signal};
use crate::p2p::bootstrap::BootstrapServer;

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

impl TrackerServer {
    async fn new(ip: String, port: u16,) -> Self {
        log::info!("Initializing TrackerServer...");
        TrackerServer{ip, port}
    }

    async fn run(&self, data_connection: (String, Vec<String>)) -> anyhow::Result<()> {
        let app = Router::new().route("/", get(|| async { "Hello from TrackerServer!" }));

        let addr_str = format!("{}:{}", self.ip, self.port);
        let addr: SocketAddr = addr_str.parse()?;
        log::info!("TrackerServer listening on http://{:?}", addr);

        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .await
            .expect("TrackerServer failed");
        Ok(())
    }
}

#[tokio::main]
async fn main() {
    // Graceful shutdown control
    let tracker_address = "0.0.0.0";
    let tracker_port = 3000;
    let tracker = TrackerServer::new(tracker_address.to_string(), tracker_port).await;

    /*  p2p bootstrap server */
    let mut p2p_bootstrap_server = BootstrapServer::new().await.unwrap();
    let data_connection = p2p_bootstrap_server.data_connection();
    select! {
        _ = tracker.run(data_connection).await.unwrap() => println!("Tracker server exited."),
        _ = p2p_bootstrap_server.run().await.unwrap(); => println!("libp2p server exited."),
        _ = signal::ctrl_c() => println!("Received Ctrl+C, shutting down."),
    }
}




