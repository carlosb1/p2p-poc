use reqwest::get;
use scraper::{Html, Selector};

async fn extract_og_metadata(url: &str) -> Option<(String, String, String)> {
    let body = get(url).await.ok()?.text().await.ok()?;
    let document = Html::parse_document(&body);
    let selector = Selector::parse("meta").ok()?;

    let mut title = None;
    let mut image = None;
    let mut desc = None;

    for element in document.select(&selector) {
        if let Some(prop) = element.value().attr("property") {
            match prop {
                "og:title" => title = element.value().attr("content").map(|s| s.to_string()),
                "og:image" => image = element.value().attr("content").map(|s| s.to_string()),
                "og:description" => desc = element.value().attr("content").map(|s| s.to_string()),
                _ => {}
            }
        }
    }

    Some((
        title.unwrap_or_default(),
        image.unwrap_or_default(),
        desc.unwrap_or_default(),
    ))
}

#[tokio::main]
pub async fn main(){
    let (title, image, description) = extract_og_metadata("https://cincodias.elpais.com/mercados-financieros/2025-08-01/la-bolsa-y-el-ibex-35.html").await.unwrap();
    println!("{:#?}", title);
    println!("{:#?}", image);
    println!("{:#?}", description);
}