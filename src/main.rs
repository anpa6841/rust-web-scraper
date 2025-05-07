use anyhow::Result;
use scraper::{Html, Selector};
use reqwest::Client;
use tokio::task;

#[tokio::main]
async fn main() -> Result<()> {
    let urls = vec![
        "https://example.com",
        "https://www.rust-lang.org",
        // Add more URLs as needed
    ];

    let client = Client::new();
    let mut tasks = vec![];

    for url in urls {
        let client = client.clone();
        tasks.push(task::spawn(async move {
            match scrape_titles(&client, &url).await {
                Ok(titles) => (url, titles),
                Err(e) => (url, vec![format!("Error: {}", e)]),
            }
        }));
    }

    for task in tasks {
        let (url, titles) = task.await?;
        println!("Titles from {}:", url);
        for title in titles {
            println!("- {}", title);
        }
    }

    Ok(())
}

async fn scrape_titles(client: &Client, url: &str) -> Result<Vec<String>> {
    let response = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&response);
    let selector = Selector::parse("h1, h2, h3").unwrap();

    let titles = document
        .select(&selector)
        .map(|element| element.inner_html().trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    Ok(titles)
}
