use dioxus::prelude::*;
use futures::{SinkExt, StreamExt};
use gloo_net::http::Request;
use gloo_net::websocket::{futures::WebSocket, Message};
use serde::{Deserialize, Serialize};

fn main() {
    launch(App);
}

#[derive(Serialize)]
pub struct SearchQuery {
    query: String,
}

#[derive(Serialize, Deserialize, Clone)]
struct LinkPayload {
    pub url: String,
    pub tags: Vec<String>,
    pub description: Option<String>,
}


#[derive(PartialEq, Clone)]
pub enum Page {
    Links,
    Interactive,
    Search,
}
#[derive(PartialEq, Props, Clone)]
struct URLBackendProps {
    url_backend: String,
}



#[component]
pub fn App() -> Element {
    let mut current_page = use_signal(|| Page::Interactive);
    let mut url_backend: Signal<String> = use_signal(move || "http://0.0.0.0:3000".to_string());

    rsx! {
        div {
            nav {
                button { onclick: move |_| current_page.set(Page::Links), "Links" }
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
                        Interactive {url_backend: url_backend.clone()}
                    },
                    Page::Links => rsx! {
                        document::Stylesheet { href: asset!("/assets/main.css") }
                        Links {url_backend: url_backend.clone()}
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





#[component]
fn Links(url_backend: Signal<String>) -> Element{
    let mut url = use_signal(|| "".to_string());
    let mut description = use_signal(|| "".to_string());
    let mut feedback = use_signal(|| None::<String>);
    let url_backend: Signal<String> = use_signal(move || "http://0.0.0.0:3000".to_string());


    let submit = move |_| {
        let url = url().clone();
        let desc = description().clone();

        spawn(async move {
            if url.trim().is_empty() {
                feedback.set(Some("URL no puede estar vacía".to_string()));
                return;
            }

            let payload = LinkPayload {
                url: url.clone(),
                description: if desc.trim().is_empty() {
                    None
                } else {
                    Some(desc.clone())
                },
                tags: vec!["tag1".to_string(), "tag2".to_string()],
            };


            let http_url_endpoint_backend = format!("http://{}/link",url_backend);

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
        });
    };

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

            button {
                onclick: submit,
                class: "button",
                "Enviar"
            }

            if let Some(msg) = feedback() {
                p { "{msg}" }
            }
        }
    }
}



#[component]
fn Search(url_backend: Signal<String>) -> Element {
    let mut query = use_signal(|| "".to_string());
    let mut feedback = use_signal(|| None::<String>);
    let mut links = use_signal(|| Vec::<LinkPayload>::new());

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
                    let new_links: Vec<LinkPayload> = res.json().await.unwrap_or_default();
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
fn Interactive(url_backend: Signal<String>) -> Element {
    let mut receiver_ws = use_signal(|| None);
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

    let mut links = use_signal(|| Vec::<LinkCardProps>::new());

    /* we receive signals from websocket and we add message */
    let _ = use_future(move || async move {
        if let Some(mut receiver) = receiver_ws.take() {
            while let Some(msg) = receiver.next().await {
                if let Ok(msg) = msg {
                    match msg {
                        Message::Text(content) => {
                            links.write().push(LinkCardProps {url:"mocked".to_string(), description:content}
                            );
                        },
                        _ => ()
                    }
                }
            }
        }
    });

    rsx! {
        h1 { "Reel" }

        for (i, card) in links.iter().enumerate() {
            div {
                class: "card",
                h3 { "{i}"}
                p { "{card.description}"}
                a {
                    href: "{card.url}",
                    target: "_blank",
                    "Ir al sitio"
                }
            }
        }
    }
}