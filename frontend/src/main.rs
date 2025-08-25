mod models;

use std::collections::HashMap;
use std::time::Duration;
use dioxus::prelude::*;
use dioxus::warnings::AllowFutureExt;
use futures::{SinkExt, StreamExt};
use gloo_net::http::Request;
use gloo_net::websocket::{futures::WebSocket, Message};
use serde::{Deserialize, Serialize};
use models::Link;
use crate::models::{Content, ContentToValidate, Reputation, Topic, Votation, Vote};

fn main() {
    launch(App);
}


#[derive(PartialEq, Clone)]
pub enum Page {
    Links,
    Interactive,
    Search,
    Validation,
}
#[derive(PartialEq, Props, Clone)]
struct URLBackendProps {
    url_backend: String,
}


/* helper functions */
pub fn colorize(tag: &str) -> (String, String) {
    // hash simple estable
    let mut hash: u32 = 2166136261;
    for b in tag.bytes() {
        hash ^= b as u32;
        hash = hash.wrapping_mul(16777619);
    }
    let hue = (hash % 360) as i32;         // 0..359
    let sat = 70;                           // %
    let light = 45;                         // %
    // color de texto según luminosidad
    let fg = if light > 60 { "#222" } else { "white" };
    (format!("hsl({} {}% {}%)", hue, sat, light), fg.into())
}

async fn post_link(mut feedback: Signal<Option<String>>, url_backend: Signal<String>, payload: &Link) {
    let http_url_endpoint_backend = format!("http://{}/link", url_backend);

    let response = Request::post(http_url_endpoint_backend.as_str())
        .header("Content-Type", "application/json")
        .json(&payload)
        .unwrap()
        .send()
        .await;

    match response {
        Ok(res) if res.status() == 200 => {
            feedback.set(Some("✅ Enviado correctamente".to_string()));
        }
        Ok(res) => {
            feedback.set(Some(format!("❌ Error: {}", res.status())));
        }
        Err(err) => {
            feedback.set(Some(format!("❌ Error de red: {} {}", http_url_endpoint_backend, err)));
        }
    }
}

async fn add_vote(mut feedback: Signal<Option<String>>, url_backend:String, payload: Vote) {
    let http_url_endpoint_backend = format!("http://{}/vote", url_backend);

    let response = Request::post(http_url_endpoint_backend.as_str())
        .header("Content-Type", "application/json")
        .json(&payload)
        .unwrap()
        .send()
        .await;

    match response {
        Ok(res) if res.status() == 200 => {
            feedback.set(Some("✅ Enviado correctamente".to_string()));
        }
        Ok(res) => {
            feedback.set(Some(format!("❌ Error: {}", res.status())));
        }
        Err(err) => {
            feedback.set(Some(format!("❌ Error de red: {} {}", http_url_endpoint_backend, err)));
        }
    }
}

async fn post_register_topic(mut feedback: Signal<Option<String>>, url_backend: Signal<String>, payload: &str) {
    let http_url_endpoint_backend = format!("http://{}/topic/register", url_backend);

    let response = Request::post(http_url_endpoint_backend.as_str())
        .header("Content-Type", "application/json")
        .json(&payload)
        .unwrap()
        .send()
        .await;

    match response {
        Ok(res) if res.status() == 200 => {
            feedback.set(Some("✅ Enviado correctamente".to_string()));
        }
        Ok(res) => {
            feedback.set(Some(format!("❌ Error: {}", res.status())));
        }
        Err(err) => {
            feedback.set(Some(format!("❌ Error de red: {} {}", http_url_endpoint_backend, err)));
        }
    }
}

async fn post_new_remote_topic(mut feedback: Signal<Option<String>>, url_backend: Signal<String>, payload: &str) {
    let http_url_endpoint_backend = format!("http://{}/topic/new", url_backend);

    let response = Request::post(http_url_endpoint_backend.as_str())
        .header("Content-Type", "application/json")
        .json(&payload)
        .unwrap()
        .send()
        .await;

    match response {
        Ok(res) if res.status() == 200 => {
            feedback.set(Some("✅ Enviado correctamente".to_string()));
        }
        Ok(res) => {
            feedback.set(Some(format!("❌ Error: {}", res.status())));
        }
        Err(err) => {
            feedback.set(Some(format!("❌ Error de red: {} {}", http_url_endpoint_backend, err)));
        }
    }
}


#[component]
pub fn App() -> Element {
    let mut current_page = use_signal(|| Page::Interactive);
    let mut url_backend: Signal<String> = use_signal(move || "0.0.0.0:3000".to_string());
    let mut links = use_signal(|| Vec::<LinkCardProps>::new());

    let mut receiver_ws = use_signal(|| None);

    // data from ws
    let mut my_topics = use_signal(|| Vec::<Topic>::new());
    let mut content= use_signal(|| Vec::<Content>::new());
    let mut my_pending_content= use_signal(|| Vec::<Content>::new());

    // list my content that I want to validate
    let mut content_to_validate = use_signal(|| Vec::<ContentToValidate>::new());
    let mut voters_by_key = use_signal(|| HashMap::<String, Votation>::new());
    let mut reputations_by_topic = use_signal(|| HashMap::<String, Vec<Reputation>>::new());


    /* web socket code */
    /* we send message via websocket */
    let ws_client = use_coroutine(move | rx:UnboundedReceiver<String>| {
        let cloned_url = url_backend.clone();
        async move {
            //TODO for sending, use sender channel
            let (mut sender, receiver) = WebSocket::open(format!("ws://{}/ws", cloned_url).as_str()).unwrap().split();
            receiver_ws.set(Some(receiver));
            sender.send(Message::Text("#start_listen".to_string())).await.unwrap();
            /*while let Some(msg) = rx.next().await {
                let message = format!("{}",msg);
                sender.send(Message::Text(message)).await.unwrap();
            }
             */
        }
    });



    /* we receive signals from websocket and we add message */
    let _ = use_future(move || async move {
        if let Some(mut receiver) = receiver_ws.take() {
            while let Some(msg) = receiver.next().await {
                if let Ok(msg) = msg {
                    match msg {
                        Message::Text(str_content) => {
                            if let Ok(payload) = serde_json::from_str::<models::WSContentData>(str_content.as_str()) {
                                // we udpate all our states
                                my_topics.set(payload.my_topics);
                                content.set(payload.content);
                                content_to_validate.set(payload.content_to_validate);
                                voters_by_key.set(payload.voters_by_key);
                                reputations_by_topic.set(payload.reputations_by_topic);
                                my_pending_content.set(payload.my_pending_content)
                            }
                            links.write().push(LinkCardProps {url:"mocked".to_string(), description:str_content}
                            );


                        },
                        _ => ()
                    }
                }
            }
        }
    });



    rsx! {
        div {
            nav {
                button { onclick: move |_| current_page.set(Page::Links), "Links" }
                button { onclick: move |_| current_page.set(Page::Validation), "Validation" }
                button { onclick: move |_| current_page.set(Page::Interactive), "Interactive" }
                button { onclick: move |_| current_page.set(Page::Search), "Search" }
                input {
                    r#type: "text",
                    placeholder: "http://...",
                    value: "{url_backend}",
                    oninput: move |e| url_backend.set(e.value()),
                    class: "input"
                }

            }

            main {
                match current_page() {
                    Page::Interactive => rsx! {
                        document::Stylesheet { href: asset!("/assets/main.css") }
                        Interactive {url_backend: url_backend.clone(), links, content}
                    },
                    Page::Links => rsx! {
                        document::Stylesheet { href: asset!("/assets/main.css") }
                        Links {url_backend: url_backend.clone(), my_topics, content_to_validate}
                    },

                   Page::Validation => rsx! {
                        document::Stylesheet { href: asset!("/assets/main.css") }
                        Validation {url_backend: url_backend.clone(), my_pending_content}
                    },

                    Page::Search => rsx! {
                        document::Stylesheet { href: asset!("/assets/main.css") }
                        Search {url_backend: url_backend.clone()}
                    },
                }
            }
        }
    }
}




// ---- Tipos de UI/estado ----

#[derive(Clone, Debug, PartialEq)]
enum LinkStatus {
    Pending,
    Ok,
    Error(String),
}

#[derive(Clone, Debug, PartialEq)]
struct SentContent {
    id: u64,                // id local para referenciar y actualizar
    url: String,
    description: Option<String>,
    tags: Vec<String>,
    status: LinkStatus,
}

#[component]
fn Links(url_backend: Signal<String>, my_topics:  Signal<Vec<Topic>>, content_to_validate: Signal<Vec<ContentToValidate>>) -> Element{
    let mut url = use_signal(|| "".to_string());
    let mut description = use_signal(|| "".to_string());
    let mut feedback = use_signal(|| None::<String>);
    let url_backend: Signal<String> = use_signal(move || "0.0.0.0:3000".to_string());

    /* tags variables */
    let mut tags = use_signal(|| Vec::<String>::new());
    let mut tag_input = use_signal(|| "".to_string());

    /* topics variables */
    // ask remotely topics
    let mut topics = use_signal(|| Vec::<String>::new());
    let mut topic_input = use_signal(|| "".to_string());

    let pending_content = use_signal(|| Vec::<SentContent>::new());


    // helper functions for adding remove or keydown
    let mut add_tag = move |raw: String| {
        let t = raw.trim().to_string();
        if t.is_empty() { return; }
        if !tags.read().iter().any(|x| x.eq_ignore_ascii_case(&t)) {
            tags.write().push(t);
        }
        tag_input.set(String::new());
    };

    let on_tag_keydown = move |ev: KeyboardEvent| {
        if (ev.key() == Key::Enter || ev.key() == Key::Character(",".to_string()))
            && (tags.is_empty()){
            ev.prevent_default();
            add_tag(tag_input());
        }
    };

    let on_add_click = move |_| {
        if tags.is_empty() {
            add_tag(tag_input());
        }
    };

    let mut remove_tag = move |idx: usize| {
        tags.write().remove(idx);
    };

    // helper functions for adding remove or keydown
    let mut add_topic = move |raw: String| {
        let t = raw.trim().to_string();
        if t.is_empty() { return; }
        if !topics.read().iter().any(|x| x.eq_ignore_ascii_case(&t)) {
            topics.write().push(t);
        }
        topic_input.set(String::new());
    };
    let mut remove_topic = move |idx: usize| {
        topics.write().remove(idx);
    };

    let on_topic_keydown = move |ev: KeyboardEvent| {
        if ev.key() == Key::Enter || ev.key() == Key::Character(",".to_string()){
            ev.prevent_default();
            add_topic(topic_input());
        }
    };

    let on_add_click_topic = move |_| {
            add_topic(topic_input());
    };

    let on_add_click_register_topic =  move |_| {
        spawn(async move {
            if let Some(topic) = topics.read().first() {
                post_register_topic(feedback,url_backend,topic).await;
            }
        });
    };

    let on_add_click_new_remote_topic =  move |_| {
        spawn(async move {
            if let Some(topic) = topics.read().first() {
                post_new_remote_topic(feedback,url_backend,topic).await;
            }
        });
    };


    let tags_for_link: Vec<(String, String, String)> = tags.read().iter().cloned().map(|t| {
        let (bg, fg) = colorize(&t);
        (t, bg, fg)
    }).collect();


    let topics_to_register: Vec<(String, String, String)> = topics.read().iter().cloned().map(|t| {
        let (bg, fg) = colorize(&t);
        (t, bg, fg)
    }).collect();

    let my_topic_nodes: Vec<(String, String, String)> = my_topics.read().iter().cloned().map(|t| {
        let (bg, fg) = colorize(&t.title);
        (t.title, bg, fg)
    }).collect();

    let status_badge = |st: &LinkStatus| -> String {
        match st {
            LinkStatus::Pending => "⏳ Pendiente".into(),
            LinkStatus::Ok      => "✅ OK".into(),
            LinkStatus::Error(_) => "❌ Error".into(),
        }
    };

    
    let submit = move |_| {
        let url = url().clone();
        let desc = description().clone();
        let tags_for_payload: Vec<String> = tags.read().iter().map(|e| e.to_ascii_lowercase()).collect();

        spawn(async move {
            if url.trim().is_empty() {
                feedback.set(Some("URL no puede estar vacía".to_string()));
                return;
            }

            let payload = Link {
                url: url.clone(),
                description: if desc.trim().is_empty() {
                    None
                } else {
                    Some(desc.clone())
                },
                tags: tags_for_payload,
            };

            post_link(feedback, url_backend, &payload).await;
        });
    };

    // UI
    rsx! {
        div {
                class: "form-container",
                h2 { "Enviar un nuevo enlace" }

                input {
                    r#type: "text",
                    placeholder: "https://...",
                    value: "{url}",
                    oninput: move |e| url.set(e.value()),
                    class: "input"
                }

                textarea {
                    placeholder: "Descripción opcional",
                    value: "{description}",
                    oninput: move |e| description.set(e.value()),
                    class: "textarea"
                }

                div { class: "tag-input-row",
                input {
                    r#type: "text",
                    placeholder: "Añade tags (Enter o ,)",
                    value: "{tag_input}",
                    oninput: move |e| tag_input.set(e.value()),
                    onkeydown: on_tag_keydown,
                    class: "input"
                }
                button { class: "button", onclick: on_add_click, "Añadir" }
            }

                div { class: "tag-list",
                for (i , (t, bg, fg)) in tags_for_link.iter().cloned().enumerate() {
                    div {
                        class: "tag",
                        style: "background-color:{bg}; color:{fg};",
                        "{t}"
                        button { class: "remove", onclick: move |_| remove_tag(i), "×" }
                    }
                }
            }

            button {
                onclick: submit,
                class: "button",
                "Enviar"
            }

            if let Some(msg) = feedback() {
                p { "{msg}" }
            }

            h3 {"Mis topics"}
            div { class: "tag-list",
                  for (i , (t, bg, fg)) in my_topic_nodes.iter().cloned().enumerate() {
                    div {
                        class: "tag",
                        style: "background-color:{bg}; color:{fg};",
                        "{t}",
                    }
                }
            }
            h3 {"Añadir topics"}
            div { class: "tag-input-row",
                input {
                    r#type: "text",
                    placeholder: "Añade tags (Enter o ,)",
                    value: "{topic_input}",
                    oninput: move |e| topic_input.set(e.value()),
                    onkeydown: on_topic_keydown,
                    class: "input"
                }
                button { class: "button", onclick: on_add_click_topic, "Añadir" }
                button { class: "button", onclick: on_add_click_register_topic, "Registrar topics" }
                button { class: "button", onclick: on_add_click_new_remote_topic, "Añadir topics" }
            }

            div { class: "tag-list",
                for (i , (t, bg, fg)) in topics_to_register.iter().cloned().enumerate() {
                    div {
                        class: "tag",
                        style: "background-color:{bg}; color:{fg};",
                        "{t}",
                        button { class: "remove", onclick: move |_| remove_topic(i), "×" }
                    }
                }
            }
            h3 {"Contenido pendiente de validar"}
            div {
            class: "link-list",
            for cont_val in content_to_validate.read().clone().iter() {
                    div {
                        class: "link-card",
                        p { "{cont_val.id_votation}"}
                        p { class: "link-desc", "{cont_val.content}" }
                        div {
                            class: "tag-list",
                            span { class: "tag", "#{cont_val.topic}" }

                        }
                    }
                }
            }
        }
    }
}



async fn post_vote(mut feedback: Signal<Option<String>>, url_backend: Signal<String>, payload: &Vote) {
    let http_url_endpoint_backend = format!("http://{}/vote", url_backend);

    let response = Request::post(http_url_endpoint_backend.as_str())
        .header("Content-Type", "application/json")
        .json(&payload)
        .unwrap()
        .send()
        .await;

    match response {
        Ok(res) if res.status() == 200 => {
            feedback.set(Some("✅ Enviado correctamente".to_string()));
        }
        Ok(res) => {
            feedback.set(Some(format!("❌ Error: {}", res.status())));
        }
        Err(err) => {
            feedback.set(Some(format!("❌ Error de red: {} {}", http_url_endpoint_backend, err)));
        }
    }
}

#[component]
fn Search(url_backend: Signal<String>) -> Element {
    let mut query = use_signal(|| "".to_string());
    let mut feedback = use_signal(|| None::<String>);
    let mut links = use_signal(|| Vec::<Link>::new());

    let submit = move |_| {
        let query = query().clone();
        let url_backend = url_backend.clone();

        spawn(async move {
            let encoded_query = urlencoding::encode(&query);
            let http_url_endpoint_backend = format!("http://{}/search?query={}",url_backend, encoded_query);

            let response = Request::get(http_url_endpoint_backend.as_str())
                .send()
                .await;

            match response {
                Ok(res) if res.status() == 200 => {
                    let new_links: Vec<Link> = res.json().await.unwrap_or_default();
                    feedback.set(Some("✅ Enviado correctamente".to_string()));
                    links.set(new_links);
                }
                Ok(res) => {
                    feedback.set(Some(format!("❌ Error: {}", res.status())));
                }
                Err(err) => {
                    feedback.set(Some(format!("❌ Error de red: {} {}", http_url_endpoint_backend, err)));
                }
            }
        });
    };

    rsx! {
        div {
            class: "form-container",
            h2 { "Buscador" }

            input {
                r#type: "text",
                placeholder: "Palabras para buscar",
                value: "{query}",
                oninput: move |e| query.set(e.value()),
                class: "input"
            }


            button {
                onclick: submit,
                class: "button",
                "Buscar"
            }

            if let Some(msg) = feedback() {
                p { "{msg}" }
            }

                    div {
            class: "link-list",
            for link in links.read().clone().iter() {
                    div {
                        class: "link-card",
                        a {
                            href: "{link.url}",
                            target: "_blank",
                            class: "link-title",
                            "{link.url}"
                        }
                        if let Some(desc) = &link.description {
                            p { class: "link-desc", "{desc}" }
                        }
                        div {
                            class: "tag-list",
                            for tag in link.tags.iter() {
                                span { class: "tag", "#{tag}" }
                            }
                        }
                    }
                }
            }
        }
    }
}



#[derive(Debug, Clone, PartialEq, Props)]
struct LinkCardProps {
    url: String,
    description: String,
}

#[component]
fn Validation(url_backend: Signal<String>,
              my_pending_content: Signal<Vec<Content>>) -> Element {

    let mut feedback = use_signal(|| None::<String>);
    let mut topic = use_signal(|| "".to_string());

    async fn on_like_vote(id_votation: String, topic: String, feedback: Signal<Option<String>>,  url_backend: String) {
        log::info!("👍 like");
        let vote = Vote {
            id: id_votation,
            topic,
            vote: true,
        };
        add_vote(feedback, url_backend, vote).await;
    }

    async fn on_dislike_vote(id_votation: String, topic: String, feedback: Signal<Option<String>>,  url_backend: String) {
        let vote = Vote {
            id: id_votation,
            topic,
            vote: true,
        };
        add_vote(feedback, url_backend, vote).await;
        log::info!("👎 no");
    }

    rsx! {
        h1 {"Para revisar"}

        div {
            class: "tag-input-row",
            input {
                r#type: "text",
                placeholder: "Escribe un topic",
                value: "{topic}",
                oninput: move |evt| topic.set(evt.value().to_string()),
            }
        }

        if let Some(msg) = feedback() {
                p { "{msg}" }
        }

        for (i, card) in my_pending_content.iter().enumerate() {
            div {
                class: "card",
                h3 { "{i}"}
                p { "{card.id_votation}"}
                p { "{card.content}"}
            div { class: "card-footer",
                // like
            button {
                class: "btn-action btn-like",
                onclick: {
                    let id = card.id_votation.clone();
                    let fb = feedback.clone();
                    let topic_sig = topic.clone();
                    move |_| {
                        // read topic to an owned String now
                        let topic_val = topic_sig.read().clone();
                        let url = url_backend.read().clone();

                        // build an OWNED vote and move it into the task
                        let vote = Vote {
                            id: id.clone(),
                            topic: topic_val,
                            vote: true,
                        };

                        spawn(async move {
                            add_vote(fb, url, vote).await; // pass owned vote
                        });
                    }
                },
                "👍 Sí"
                }

                // dislike
                button {
                    class: "btn-action btn-dislike",
                onclick: {
                    let id = card.id_votation.clone();
                    let fb = feedback.clone();
                    let topic_sig = topic.clone();

                    move |_| {
                        // read topic to an owned String now
                        let topic_val = topic_sig.read().clone();
                        let url = url_backend.read().clone();
                        // build an OWNED vote and move it into the task
                        let vote = Vote {
                            id: id.clone(),
                            topic: topic_val,
                            vote: true,
                        };

                        spawn(async move {
                            add_vote(fb, url, vote).await; // pass owned vote
                        });
                    }
                },
                "👎 No"
                }
            }
            }
        }
    }

}

#[component]
fn Interactive(url_backend: Signal<String>,
               links: Signal<Vec<LinkCardProps>>,
               content: Signal<Vec<Content>>) -> Element {
    rsx! {
        h1 { "Reel" }
            div {
            //    class: "parent-grid-cards",
                for (i, card) in content.iter().enumerate() {
                    div {
                        class: "card",
                        h3 { "{i}"}
                        p { "{card.id_votation}"}
                        p { "{card.content}"}
                        if card.approved {
                          p { span { "✅" } }
                        }
                    }
                }
            }
    }
}