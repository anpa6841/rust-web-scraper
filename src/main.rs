use anyhow::{Result, anyhow};
use scraper::{Html, Selector};
use reqwest::Client;
use tokio::task;
use chrono::{DateTime, NaiveDateTime, Utc};
use url::Url; // Add url crate for parsing and joining URLs

#[derive(Debug)]
struct Article {
    title: String,
    url: String,
    pub_date: Option<DateTime<Utc>>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let urls = vec![
        "http://example.com",
        "http://books.toscrape.com",
        // Add more URLs for testing, e.g., "http://quotes.toscrape.com"
    ];

    let client = Client::new();
    let mut tasks = vec![];

    for url in urls {
        let client = client.clone();
        tasks.push(task::spawn(async move {
            match scrape_articles(&client, &url).await {
                Ok(articles) => (url, articles),
                Err(e) => (url, vec![Article {
                    title: format!("Error: {}", e),
                    url: String::new(),
                    pub_date: None,
                }]),
            }
        }));
    }

    for task in tasks {
        let (url, articles) = task.await?;
        println!("Articles from {}:", url);
        for article in articles {
            println!(
                "- {} (URL: {}, Published: {})",
                article.title,
                article.url,
                article.pub_date.map_or("Unknown".to_string(), |d| d.to_string())
            );
        }
    }

    Ok(())
}

async fn scrape_articles(client: &Client, url: &str) -> Result<Vec<Article>> {
    // Parse the input URL to extract the base URL for joining relative links
    let base_url = Url::parse(url).map_err(|_| anyhow!("Invalid URL: {}", url))?;

    let response = client.get(url).send().await?.text().await?;
    let document = Html::parse_document(&response);

    // Selectors for books (compatible with books.toscrape.com, adjustable for other sites)
    let article_selector = Selector::parse("article.product_pod").map_err(|_| anyhow!("Invalid selector"))?;
    let title_selector = Selector::parse("h3 a").map_err(|_| anyhow!("Invalid title selector"))?;
    let link_selector = Selector::parse("h3 a").map_err(|_| anyhow!("Invalid link selector"))?;

    let mut articles = vec![];

    println!("Scraping URL: {}", url);
    for element in document.select(&article_selector) {
        // Extract title
        let title = element
            .select(&title_selector)
            .next()
            .and_then(|t| t.value().attr("title"))
            .map(|t| t.to_string())
            .unwrap_or("Untitled".to_string());
        println!("Title: {}", title);

        // Extract URL
        let url = element
            .select(&link_selector)
            .next()
            .and_then(|a| a.value().attr("href"))
            .map(|href| {
                // Join relative URL with base URL
                base_url.join(href).map(|u| u.to_string()).unwrap_or_default()
            })
            .unwrap_or_default();
        println!("URL: {}", url);

        // No date available on books.toscrape.com, so set pub_date to None
        let pub_date = None;

        if !title.is_empty() {
            articles.push(Article { title, url, pub_date });
        }
    }

    Ok(articles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    use wiremock::matchers::{method, path};

    #[tokio::test]
    async fn test_scrape_articles() -> Result<()> {
        // Set up mock server
        let mock_server = MockServer::start().await;
        let mock_html = r#"
            <article class="product_pod">
                <h3><a href="/catalogue/test-book_1/index.html" title="Test Book">Test Book</a></h3>
            </article>
        "#;
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(mock_html))
            .mount(&mock_server)
            .await;

        // Test scraping
        let client = Client::new();
        let articles = scrape_articles(&client, &mock_server.uri()).await?;

        assert_eq!(articles.len(), 1);
        assert_eq!(articles[0].title, "Test Book");
        assert_eq!(articles[0].url, format!("{}/catalogue/test-book_1/index.html", mock_server.uri()));
        assert!(articles[0].pub_date.is_none());
        Ok(())
    }

    #[tokio::test]
    async fn test_scrape_articles_no_articles() -> Result<()> {
        let mock_server = MockServer::start().await;
        let mock_html = "<div>No articles here</div>";
        Mock::given(method("GET"))
            .and(path("/"))
            .respond_with(ResponseTemplate::new(200).set_body_string(mock_html))
            .mount(&mock_server)
            .await;

        let client = Client::new();
        let articles = scrape_articles(&client, &mock_server.uri()).await?;
        assert_eq!(articles.len(), 0);
        Ok(())
    }
}
