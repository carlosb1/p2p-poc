use std::thread::Scope;
use dioxus::prelude::*;

use dioxus::prelude::*;

#[derive(Clone, Debug, PartialEq)]
struct Card {
    title: &'static str,
    description: &'static str,
    image_url: &'static str,
    upvotes: usize,
    downvotes: usize,
}

#[component]
fn App() -> Element {

    let mut cards = use_signal(|| {
        vec![
            Card {
                title: "Rust",
                description: "Rust is a fast and safe system programming language.",
                image_url: "https://www.rust-lang.org/logos/rust-logo-512x512.png",
                upvotes: 0,
                downvotes: 0,
            },
            Card {
                title: "Dioxus",
                description: "Dioxus is a modern UI library for Rust apps.",
                image_url: "https://raw.githubusercontent.com/DioxusLabs/assets/main/logos/dioxus-full.png",
                upvotes: 0,
                downvotes: 0,
            },
            Card {
                title: "WASM",
                description: "WebAssembly runs high-perf code in browsers.",
                image_url: "https://upload.wikimedia.org/wikipedia/commons/1/1f/WebAssembly_Logo.svg",
                upvotes: 0,
                downvotes: 0,
            },
        ]
    });

    let mut dragged_index = use_signal(|| None);

    rsx! {
        link {
            rel: "stylesheet",
            href: "https://cdn.jsdelivr.net/npm/tailwindcss@2.2.19/dist/tailwind.min.css",
        }

        div { class: "min-h-screen bg-gradient-to-r from-gray-100 to-blue-100 p-10",
            h1 { class: "text-4xl font-bold text-center text-gray-800 mb-10", "Arrastra, Vota y Organiza" }

            div { class: "grid grid-cols-1 md:grid-cols-3 gap-6 max-w-6xl mx-auto",
                for (i, card) in cards.iter().enumerate() {
                    div {
                        key: "{i}",
                        draggable: "true",
                        ondragstart: move |_| { dragged_index.clone().set(Some(i)) },
                        ondragover: move |e| { e.prevent_default(); },
                        ondrop: move |_| {
                            let mut vec = cards.read().clone();

                            if let Some(from_i) = *dragged_index.read() {
                                if from_i != i && from_i < vec.len() && i < vec.len() {
                                    vec.swap(from_i, i);
                                    cards.set(vec);
                                }
                            }

                            dragged_index.set(None);
                        },
                        class: "bg-white rounded-xl shadow-lg hover:shadow-2xl transition-shadow duration-300 cursor-move",
                        img { class: "w-full h-48 object-cover rounded-t-xl", src: "{card.image_url}" }
                        div { class: "p-6",
                            h2 { class: "text-2xl font-semibold text-gray-800", "{card.title}" }
                            p { class: "text-gray-600 mt-3", "{card.description}" }

                            div { class: "mt-4 flex space-x-4",
                                button {
                                    class: "flex items-center px-3 py-1 bg-green-100 hover:bg-green-200 text-green-800 rounded-full",
                                    onclick: move |_| {
                                        cards.write().get_mut(i).unwrap().upvotes+=1;
                                    },
                                    "👍 {cards.read()[i].upvotes}"
                                }
                                button {
                                    class: "flex items-center px-3 py-1 bg-red-100 hover:bg-red-200 text-red-800 rounded-full",
                                    onclick: move |_| {
                                         cards.write().get_mut(i).unwrap().downvotes += 1;
                                    },
                                    "👎 {cards.read()[i].downvotes}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}



fn main() {
    #[cfg(feature = "web")]
    // Hydrate the application on the client
    dioxus::launch(App);

    // Launch axum on the server
    #[cfg(feature = "server")]
    {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                launch_server(App).await;
            });
    }
}
#[cfg(feature = "server")]
async fn launch_server(component: fn() -> Element) {
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    // Get the address the server should run on. If the CLI is running, the CLI proxies fullstack into the main address
    // and we use the generated address the CLI gives us
    let ip =
        dioxus::cli_config::server_ip().unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let port = dioxus::cli_config::server_port().unwrap_or(8080);
    let address = SocketAddr::new(ip, port);
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    let router = axum::Router::new()
        // serve_dioxus_application adds routes to server side render the application, serve static assets, and register server functions
        .serve_dioxus_application(ServeConfigBuilder::default(), App)
        .into_make_service();
    axum::serve(listener, router).await.unwrap();
}

#[server]
async fn save_dog(image: String) -> Result<(), ServerFnError> {
    Ok(())
}