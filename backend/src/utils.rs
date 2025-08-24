use std::collections::HashMap;
use crate::model::{Votation, WSContentData, Reputation, Topic, Content, ContentToValidate};
use crate::services::p2p::P2PClient;
use std::{future::Future, sync::Arc, time::Duration};
use tokio::time::{timeout, sleep};

// ---------- tiny generic helper (timeout + retries) ----------
async fn with_timeout_retries<T, E, Fut, Op>(
    dur: Duration,
    mut retries: usize,
    mut backoff: Duration,
    mut op: Op,
) -> Option<T>
where
    Op: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    loop {
        match timeout(dur, op()).await {
            Ok(Ok(v)) => return Some(v),
            Ok(Err(_e)) => { /* upstream error; retry */ }
            Err(_elapsed) => { /* timed out; retry */ }
        }
        if retries == 0 {
            return None;
        }
        retries -= 1;
        sleep(backoff).await;
        // simple exponential backoff
        backoff = backoff.saturating_mul(2);
    }
}

pub async fn fetch_data(client: Arc<P2PClient>) -> WSContentData {
    // tune these per your SLA
    const OP_TIMEOUT: Duration = Duration::from_secs(3);
    const RETRIES: usize = 2;
    const BACKOFF: Duration = Duration::from_millis(200);

    // 1) Remote-ish async calls: bound them
    let content_to_validate = with_timeout_retries(
        OP_TIMEOUT, RETRIES, BACKOFF,
        || async { Ok::<_, ()>(client.get_runtime_content_to_validate().await) }
    ).await.unwrap_or_else(Vec::new);

    let my_topics = with_timeout_retries(
        OP_TIMEOUT, RETRIES, BACKOFF,
        || async { Ok::<_, ()>(client.get_my_topics().await) }
    ).await.unwrap_or_else(Vec::new);
    
    // 2) Local/in-memory calls (assumed non-blocking)
    let content = client.all_content();

    // 3) Build reputations map (assumed cheap; if remote, add its own timeout)
    let mut reputations_by_topic: HashMap<String, Vec<Reputation>> = HashMap::new();
    for topic in my_topics.iter().cloned() {
        // if this can block (remote), wrap in with_timeout_retries similarly
        if let Ok(reputations) = client.get_reputations(topic.title.clone()) {
            let ws_reps: Vec<Reputation> = reputations.iter()
                .map(|(peer_id, score)| Reputation { peer_id: peer_id.clone(), reputation: *score })
                .collect();
            reputations_by_topic.insert(topic.title.clone(), ws_reps);
        }
    }

    // 4) Build voters_by_key (assumed local)
    let mut voters_by_key: HashMap<String, Votation> = HashMap::new();
    for content_val in content_to_validate.iter().cloned() {
        let key = content_val.id_votation.clone();
        if let Some(votation) = client.get_status_votes(key.clone()) {
            let ws_votation = Votation{
                id_votation: votation.id_votation,
                timestamp: votation.timestamp.timestamp(),
                content: votation.content,
                status: votation.status,
                leader_id: votation.leader_id,
                my_role: votation.my_role,
                votes_id: votation.votes_id.clone(),
            };
            voters_by_key.insert(key, ws_votation);
        }
    }

    WSContentData {
        my_topics,
        content,
        content_to_validate,
        voters_by_key,
        reputations_by_topic,
    }
}

pub fn fake_ws_content_data() -> WSContentData {
    let my_topics = vec![
        Topic { title: "topic-1".into(), description: "1".to_string() },
        Topic { title: "topic-2".into(), description: "2".to_string() },
    ];

    let content = vec![
        Content {
            id_votation: "content-1".into(),
            content: "hello world".into(),
            approved: false,
        },
        Content {
            id_votation: "content-2".into(),
            content: "another payload".into(),
            approved: true,
        },
    ];

    let content_to_validate = vec![
        ContentToValidate {
            id_votation: "vote-1".into(),
            topic: "topic-1".to_string(),
            content: "pending-content".into(),
            duration: Default::default(),
        },
    ];

    let mut reputations_by_topic = HashMap::new();
    reputations_by_topic.insert(
        "topic-1".into(),
        vec![
            Reputation { peer_id: "peerA".into(), reputation: 10. },
            Reputation { peer_id: "peerB".into(), reputation: -2. },
        ],
    );

    let mut voters_by_key = HashMap::new();
    voters_by_key.insert(
        "vote-1".into(),
        Votation {
            id_votation: "vote-1".into(),
            timestamp: 1_725_000_000, // fake UNIX ts
            content: "pending-content".into(),
            status: "open".into(),
            leader_id: "peerA".into(),
            my_role: "validator".into(),
            votes_id: vec![(String::from("peerA"), Some(1.)), (String::from("peerB"), Some(-2.))],
        },
    );

    WSContentData {
        my_topics,
        content,
        content_to_validate,
        voters_by_key,
        reputations_by_topic,
    }
}

