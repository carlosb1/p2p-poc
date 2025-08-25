use libp2p::{identity, PeerId};
use messages_p2p::p2p::api::APIClient;
use messages_p2p::p2p::bootstrap::BootstrapServer;
use messages_types::ChatCommand;
use protocol_p2p::db;
use protocol_p2p::models::db::Topic;
use protocol_p2p::models::messages::Vote;
use rand::distr::Alphanumeric;
use rand::{thread_rng, Rng};
use std::str::FromStr;
use tokio::task::JoinHandle;
use tokio::time::{sleep, Instant};

pub fn init_logging() {
    let _ = env_logger::builder()
        .is_test(false)
        .filter_level(log::LevelFilter::Info)
        .try_init();
    log::info!("Logging initialized for Client");
}

#[tokio::test]
async fn validation_among_clients() {
    init_logging();
    let mut clients = vec![];

    for iden_peer in 0..2 {
        let name_peer = format!("peer_id{}", iden_peer);
        std::fs::remove_dir_all(name_peer.clone());
        let keypair = identity::Keypair::generate_ed25519();
        let client =
            APIClient::new(keypair, Some(name_peer)).expect("Failed to create test client");
        clients.push(client);
    }

    let client_asker = clients.first().unwrap().clone();

    // Lanzamos todos los nodos
    let mut join_handles = vec![];
    for client in &clients {
        let (t1_handler, t2_handler) = client
            .clone()
            .start()
            .await
            .expect("Failed to start client");
        join_handles.push(t1_handler);
        join_handles.push(t2_handler);
    }

    let topic_to_register = "chat-room";

    // Todos se registran al topic
    for client in &clients {
        let topic = Topic {
            name: topic_to_register.to_string(),
            description: "example for description".to_string(),
        };
        client.register_topic(&topic).await.unwrap();
        sleep(Duration::from_secs(1)).await
    }

    log::debug!("Confirming we are registered");
    // Envían mensajes
    let random_string: String = thread_rng()
        .sample_iter(&Alphanumeric)
        .take(32)
        .map(char::from)
        .collect();

    for (i, client) in clients.iter().enumerate() {
        let msg = format!("hello world {} {}", i, random_string);
        client
            .sender()
            .send(ChatCommand::Publish(
                topic_to_register.to_string(),
                msg.clone().into_bytes(),
            ))
            .await
            .unwrap();

        client
            .sender()
            .send(ChatCommand::Publish(
                "chat-room".to_string(),
                msg.into_bytes(),
            ))
            .await
            .unwrap();
    }

    // Solicita validación
    let key = client_asker
        .new_key_available(topic_to_register, "my second content")
        .unwrap();

    client_asker
        .validate_content(&key, topic_to_register, "my second content")
        .await
        .unwrap();

    // Observa estado durante 20s
    use std::time::{Duration, Instant};
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        println!("Tick at {:?}", start.elapsed());
        tokio::time::sleep(Duration::from_secs(1)).await;

        //     let status = client_asker.get_status_vote(&key).unwrap();
        //     println!("Status: {:?}", status);

        //     let reps = client_asker.get_reputations(topic_to_register);
        //      println!("Reputs: {:?}", reps);
    }

    println!("✅ Waiting 10 secs");

    tokio::time::sleep(Duration::from_secs(10)).await;

    println!("✅ Done after 20 seconds");

    // Espera a que los tasks terminen (por limpieza)
    for handle in join_handles {
        let _ = handle.abort(); // Opcional: abortar o dejar vivir
    }
}

pub fn run_relay_server(
    keypair: identity::Keypair,
    listen_ons: Vec<String>,
    topics: Vec<String>,
    p2p_port: i32,
) -> JoinHandle<()> {
    tokio::spawn(async move {
        match BootstrapServer::new(keypair, listen_ons, topics, p2p_port).await {
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
async fn validation_among_clients_with_local_server() {
    init_logging();

    /* topic */
    let topic_to_register = "topic13".to_string();

    /* server params*/

    let p2p_port = 15000;
    let all_address = format!("/ip4/0.0.0.0/tcp/{p2p_port}").to_string();
    let loopback_address = format!("/ip4/127.0.0.1/tcp/{p2p_port}").to_string();
    let listen_ons = vec![all_address.clone(), loopback_address.clone()];
    let keypair = identity::Keypair::generate_ed25519();
    let peer_id_server = PeerId::from(keypair.public());

    /* Running relay server */

    run_relay_server(
        keypair,
        listen_ons,
        vec![topic_to_register.clone()],
        p2p_port,
    );

    let mut clients = vec![];
    /* client params */

    sleep(Duration::from_secs(10)).await;
    //////////////////////////////

    for iden_peer in 0..7 {
        let name_peer = format!("peer_id{}", iden_peer);
        let _ = std::fs::remove_dir_all(name_peer.clone());
        let keypair_client = identity::Keypair::generate_ed25519();
        let client = APIClient::from_server_params(
            keypair_client,
            Some(name_peer),
            &peer_id_server.to_string(),
            loopback_address.clone().as_str(),
        )
        .unwrap();
        clients.push(client);
    }

    let client_asker = clients.first_mut().unwrap().clone();

    let mut join_handles = vec![];
    for client in clients.clone() {
        let (t1_handler, t2_handler) = client.start().await.expect("Failed to start client");
        join_handles.push(t1_handler);
        join_handles.push(t2_handler);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    let topic = Topic {
        name: topic_to_register.clone(),
        description: "mocked description".to_string(),
    };
    client_asker.remote_new_topic(&topic).await.unwrap();

    for client in clients.clone() {
        client.register_topic(&topic).await.unwrap();
        sleep(Duration::from_secs(1)).await
    }

    log::debug!("Awaiting 10 secs getting new key");

    tokio::time::sleep(Duration::from_secs(10)).await;
    let key = client_asker
        .new_key_available(topic_to_register.as_str(), "my second content")
        .unwrap();

    log::debug!("Awaiting 10 secs for validation");
    tokio::time::sleep(Duration::from_secs(10)).await;
    client_asker
        .validate_content(&key, topic_to_register.as_str(), "my second content")
        .await
        .unwrap();

    use std::time::Duration;

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        println!("Tick at {:?}", start.elapsed());
        println!("key={:?}", key.clone());
        tokio::time::sleep(Duration::from_secs(1)).await;
        let voters = db::get_voters(&client_asker.db, &key, &topic_to_register);
        println!("voters: {:?}", voters);
        let status = client_asker.get_status_vote(&key);
        println!("Status: {:?}", status);
        let reps = client_asker.get_reputations(topic_to_register.as_str());
        println!("Reputs: {:?}", reps);
    }

    println!("All clients initialized, starting voting process...");

    for client in clients.clone() {
        println!(
            "-> Client {} {} is voting for key: {:?} and topic: {:?}",
            client.peer_id,
            client.peer_id.to_string(),
            key,
            topic_to_register
        );
        client
            .add_vote(&key, &topic_to_register, Vote::Yes)
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_secs(4)).await;
    }

    println!("✅ Waiting 10 secs");

    tokio::time::sleep(Duration::from_secs(10)).await;

    println!("✅ Done after 20 seconds");

    for client in clients.clone() {
        println!("Validating contents in peer database...");
        for content in db::get_contents(&client.db) {
            println!("Validated content: {:?}", content);
        }
    }

    // Espera a que los tasks terminen (por limpieza)
    for handle in join_handles {
        let _ = handle.abort(); // Opcional: abortar o dejar vivir
    }
}

#[tokio::test]
#[ignore] //it depends on a configured remote server
async fn validation_among_clients_with_remote_server() {
    init_logging();
    let server_address =
        "/ip4/34.244.185.56/tcp/15000/p2p/12D3KooWRCFGzavggUKszQvtUPBMnW887aE4YhYM8eYZej9DcxVM"
            .to_string();
    let peer_id_server =
        PeerId::from_str("12D3KooWRCFGzavggUKszQvtUPBMnW887aE4YhYM8eYZej9DcxVM").unwrap();

    let mut clients = vec![];
    /* client params */

    /* topic */
    let topic_to_register = "topic13".to_string();

    sleep(Duration::from_secs(10)).await;
    //////////////////////////////

    for iden_peer in 0..7 {
        let name_peer = format!("peer_id{}", iden_peer);
        let _ = std::fs::remove_dir_all(name_peer.clone());
        let keypair_client = identity::Keypair::generate_ed25519();
        let client = APIClient::from_server_params(
            keypair_client,
            Some(name_peer),
            &peer_id_server.to_string(),
            server_address.clone().as_str(),
        )
        .unwrap();
        clients.push(client);
    }

    let client_asker = clients.first_mut().unwrap().clone();

    let mut join_handles = vec![];
    for client in clients.clone() {
        let (t1_handler, t2_handler) = client.start().await.expect("Failed to start client");
        join_handles.push(t1_handler);
        join_handles.push(t2_handler);
        tokio::time::sleep(Duration::from_secs(1)).await;
    }

    let topic = Topic {
        name: topic_to_register.clone(),
        description: "mocked description".to_string(),
    };
    client_asker.remote_new_topic(&topic).await.unwrap();

    for client in clients.clone() {
        client.register_topic(&topic).await.unwrap();
        sleep(Duration::from_secs(1)).await
    }

    log::debug!("Awaiting 10 secs getting new key");

    tokio::time::sleep(Duration::from_secs(10)).await;
    let key = client_asker
        .new_key_available(topic_to_register.as_str(), "my second content")
        .unwrap();

    log::debug!("Awaiting 10 secs for validation");
    tokio::time::sleep(Duration::from_secs(10)).await;
    client_asker
        .validate_content(&key, topic_to_register.as_str(), "my second content")
        .await
        .unwrap();

    use std::time::Duration;

    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(20) {
        println!("Tick at {:?}", start.elapsed());
        println!("key={:?}", key.clone());
        tokio::time::sleep(Duration::from_secs(1)).await;
        let voters = db::get_voters(&client_asker.db, &key, &topic_to_register);
        println!("voters: {:?}", voters);
        let status = client_asker.get_status_vote(&key);
        println!("Status: {:?}", status);
        let reps = client_asker.get_reputations(topic_to_register.as_str());
        println!("Reputs: {:?}", reps);
    }

    println!("All clients initialized, starting voting process...");

    for client in clients.clone() {
        let pending_content = client.get_my_pending_to_contents_to_validate().await;
        for con in pending_content {
            println!("pending content: {:?}", con);
        }
        println!(
            "-> Client {} {} is voting for key: {:?} and topic: {:?}",
            client.peer_id,
            client.peer_id.to_string(),
            key,
            topic_to_register
        );
        client
            .add_vote(&key, &topic_to_register, Vote::Yes)
            .await
            .unwrap();
        tokio::time::sleep(Duration::from_secs(4)).await;
    }

    println!("✅ Waiting 10 secs");

    tokio::time::sleep(Duration::from_secs(10)).await;

    println!("✅ Done after 20 seconds");

    for client in clients.clone() {
        println!("Validating contents in peer database...");
        for content in db::get_contents(&client.db) {
            println!("Validated content: {:?}", content);
        }
    }

    // Espera a que los tasks terminen (por limpieza)
    for handle in join_handles {
        let _ = handle.abort(); // Opcional: abortar o dejar vivir
    }
}

#[tokio::test]
async fn api_operations_workflow() {
    init_logging();

    /* topic */
    let topic_to_register = "topic1".to_string();

    /* server params*/

    let p2p_port = 15000;
    let all_address = format!("/ip4/0.0.0.0/tcp/{p2p_port}").to_string();
    let loopback_address = format!("/ip4/127.0.0.1/tcp/{p2p_port}").to_string();
    let listen_ons = vec![all_address.clone(), loopback_address.clone()];
    let keypair = identity::Keypair::generate_ed25519();
    let peer_id_server = PeerId::from(keypair.public());

    /* Running relay server */

    run_relay_server(
        keypair,
        listen_ons,
        vec![topic_to_register.clone()],
        p2p_port,
    );

    /* */
    let name_peer = "peer_api_test";
    let _ = std::fs::remove_dir_all(name_peer.clone());
    let keypair_client = identity::Keypair::generate_ed25519();
    let client = APIClient::from_server_params(
        keypair_client,
        Some(name_peer.to_string()),
        &peer_id_server.to_string(),
        loopback_address.clone().as_str(),
    )
    .unwrap();

    /* Initialized state */
    let get_my_topics = client.get_my_topics().await;
    let content = client.all_content();
    let registered_reputations = client.get_reputations(topic_to_register.as_str());
    let pending_content_to_validate = client.get_status_voteses();

    println!("get my topics: {:?}", get_my_topics);
    println!("get content: {:?}", content);
    println!("registered reputations: {:?}", registered_reputations);
    println!(
        "pending content to validate: {:?}",
        pending_content_to_validate
    );

    client
        .remote_new_topic(&Topic::new("topic2", "description2"))
        .await
        .unwrap();
    client
        .register_topic(&Topic::new("topic5", "description5"))
        .await
        .unwrap();
    let key = client
        .new_key_available("topic3", "my second content")
        .unwrap();

    client
        .validate_content(&key, topic_to_register.as_str(), "my second content")
        .await
        .unwrap();

    /* Initialized state */
    let get_my_topics = client.get_my_topics().await;
    let content = client.all_content();
    let registered_reputations = client.get_reputations(topic_to_register.as_str());
    let pending_content_to_validate = client.get_status_voteses();
    let runtime_content_to_validate = client.get_runtime_content_to_validate().await;

    println!("get my topics: {:?}", get_my_topics);
    println!("get content: {:?}", content);
    println!("registered reputations: {:?}", registered_reputations);
    println!(
        "pending content to validate: {:?}",
        pending_content_to_validate
    );
    println!(
        "runtime pending content to validate: {:?}",
        runtime_content_to_validate
    );
    assert_eq!(get_my_topics.len(), 1);
    assert_eq!(runtime_content_to_validate.len(), 1);
}
