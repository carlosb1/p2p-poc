use libp2p::identity;
use messages_p2p::p2p;
use messages_p2p::p2p::bootstrap::BootstrapServer;
use messages_types::ChatCommand;
use rand::Rng;
use std::time::Duration;
use tokio::task::JoinHandle;

pub fn init_logging() {
    let _ = env_logger::builder()
        .is_test(false)
        .filter_level(log::LevelFilter::Debug)
        .try_init();
    log::info!("Logging initialized for Client");
}

pub fn generate_rand_msg() -> String {
    let mut rng = rand::rng();
    let random_number: u32 = rng.random_range(0..10000);
    format!("Random message: {random_number}")
}

pub fn run_relay_server() -> JoinHandle<()> {
    tokio::spawn(async {
        let listen_ons = vec![
            "/ip4/0.0.0.0/tcp/0".to_string(),
            "/ip4/127.0.0.1/tcp/34291".to_string(),
            "/ip4/0.0.0.0/tcp/34291".to_string(),
        ];
        match BootstrapServer::new(identity::Keypair::generate_ed25519(), listen_ons, 34291).await {
            Ok(mut server) => {
                if let Err(e) = server.run().await {
                    eprintln!("Server run failed: {e:?}");
                }
            }
            Err(e) => {
                eprintln!("Failed to start server: {e:?}");
            }
        }
    })
}

#[tokio::test]
async fn simple_node_communication() {
    init_logging();
    //let join_handle = run_relay_server();
    println!("Relay server started, waiting for 5 seconds to stabilize...");
    tokio::time::sleep(Duration::from_secs(5)).await;

    let node = p2p::node::run_node().unwrap();
    let tx1 = node.lock().await.command_sender();

    let node2 = p2p::node::run_node().unwrap();
    let tx2 = node2.lock().await.command_sender();

    tokio::time::sleep(Duration::from_secs(2)).await;
    let _ = tx1
        .send(ChatCommand::Publish(
            "chat-room".to_string(),
            generate_rand_msg().into_bytes(),
        ))
        .await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    tokio::time::sleep(Duration::from_secs(2)).await;
    let _ = tx2
        .send(ChatCommand::Publish(
            "chat-room".to_string(),
            generate_rand_msg().into_bytes(),
        ))
        .await;
    let _ = tx2
        .send(ChatCommand::Publish(
            "chat-room".to_string(),
            generate_rand_msg().into_bytes(),
        ))
        .await;
    tokio::time::sleep(Duration::from_secs(2)).await;

    //    let _ = join_handle;
}
