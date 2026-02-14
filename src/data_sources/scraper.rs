//! Web Scraper Module
//!
//! Integration with Firecrawl and other scraping tools

use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::time::Duration;

/// Firecrawl API client
pub struct FirecrawlClient {
    base_url: String,
    api_key: Option<String>,
    client: Client,
}

impl FirecrawlClient {
    /// Create new Firecrawl client
    pub fn new(base_url: impl Into<String>, api_key: Option<String>) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .expect("Failed to create HTTP client");
        
        Self {
            base_url: base_url.into(),
            api_key,
            client,
        }
    }
    
    /// Create default client (self-hosted)
    pub fn default_self_hosted() -> Self {
        Self::new("http://localhost:3002", None)
    }
    
    /// Scrape single URL
    pub async fn scrape(&self, url: &str, options: ScrapeOptions) -> anyhow::Result<ScrapeResult> {
        let mut request = json!({
            "url": url,
            "formats": options.formats.unwrap_or_else(|| vec!["markdown".to_string()]),
        });
        
        if let Some(only_main_content) = options.only_main_content {
            request["onlyMainContent"] = json!(only_main_content);
        }
        
        if let Some(include_tags) = options.include_tags {
            request["includeTags"] = json!(include_tags);
        }
        
        if let Some(exclude_tags) = options.exclude_tags {
            request["excludeTags"] = json!(exclude_tags);
        }
        
        let mut req = self.client
            .post(format!("{}/v1/scrape", self.base_url))
            .json(&request);
        
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        
        let response = req.send().await?;
        let status = response.status();
        
        if !status.is_success() {
            let text = response.text().await?;
            return Err(std::io::Error::other(format!("Firecrawl scrape failed: HTTP {} - {}", status, text)).into());
        }
        
        let result: FirecrawlScrapeResponse = response.json().await?;
        
        if !result.success {
            return Err(std::io::Error::other(format!("Firecrawl scrape failed: {}", 
                result.error.unwrap_or_else(|| "Unknown error".to_string()))).into());
        }
        
        Ok(ScrapeResult {
            markdown: result.data.markdown,
            html: result.data.html,
            raw_html: result.data.raw_html,
            links: result.data.links,
            metadata: result.data.metadata,
            screenshot: result.data.screenshot,
        })
    }
    
    /// Crawl entire website
    pub async fn crawl(&self, url: &str, options: CrawlOptions) -> anyhow::Result<CrawlResult> {
        let request = json!({
            "url": url,
            "limit": options.limit.unwrap_or(100),
            "maxDepth": options.max_depth,
            "includePaths": options.include_paths,
            "excludePaths": options.exclude_paths,
            "scrapeOptions": {
                "formats": options.formats.unwrap_or_else(|| vec!["markdown".to_string()]),
            }
        });
        
        let mut req = self.client
            .post(format!("{}/v1/crawl", self.base_url))
            .json(&request);
        
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        
        let response = req.send().await?;
        let status = response.status();
        
        if !status.is_success() {
            let text = response.text().await?;
            return Err(std::io::Error::other(format!("Firecrawl crawl failed: HTTP {} - {}", status, text)).into());
        }
        
        let result: FirecrawlCrawlResponse = response.json().await?;
        
        if !result.success {
            return Err(std::io::Error::other(format!("Firecrawl crawl failed: {}", 
                result.error.unwrap_or_else(|| "Unknown error".to_string()))).into());
        }
        
        Ok(CrawlResult {
            job_id: result.id,
            url: result.url,
        })
    }
}

/// Scraper service
pub struct ScraperService {
    firecrawl: Option<FirecrawlClient>,
}

impl ScraperService {
    /// Create new scraper service
    pub fn new(firecrawl_url: Option<String>, firecrawl_key: Option<String>) -> Self {
        let firecrawl = firecrawl_url.map(|url| {
            FirecrawlClient::new(url, firecrawl_key)
        });
        
        Self { firecrawl }
    }
    
    /// Check if Firecrawl is available
    pub async fn health_check(&self) -> bool {
        if let Some(ref client) = self.firecrawl {
            // Simple health check
            match client.client.get(format!("{}/health", client.base_url)).send().await {
                Ok(resp) => resp.status().is_success(),
                Err(_) => false,
            }
        } else {
            false
        }
    }
}

/// Scrape options
#[derive(Debug, Default)]
pub struct ScrapeOptions {
    pub formats: Option<Vec<String>>,
    pub only_main_content: Option<bool>,
    pub include_tags: Option<Vec<String>>,
    pub exclude_tags: Option<Vec<String>>,
    pub headers: Option<std::collections::HashMap<String, String>>,
}

/// Scrape result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeResult {
    pub markdown: Option<String>,
    pub html: Option<String>,
    pub raw_html: Option<String>,
    pub links: Option<Vec<String>>,
    pub metadata: Option<ScrapeMetadata>,
    pub screenshot: Option<String>,
}

/// Scrape metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrapeMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub source_url: String,
    pub status_code: i32,
}

/// Crawl options
#[derive(Debug, Default)]
pub struct CrawlOptions {
    pub limit: Option<i32>,
    pub max_depth: Option<i32>,
    pub include_paths: Option<Vec<String>>,
    pub exclude_paths: Option<Vec<String>>,
    pub formats: Option<Vec<String>>,
}

/// Crawl result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrawlResult {
    pub job_id: String,
    pub url: String,
}

/// Firecrawl scrape response
#[derive(Debug, Clone, Deserialize)]
struct FirecrawlScrapeResponse {
    pub success: bool,
    pub data: FirecrawlScrapeData,
    pub error: Option<String>,
}

/// Firecrawl scrape data
#[derive(Debug, Clone, Deserialize)]
struct FirecrawlScrapeData {
    pub markdown: Option<String>,
    pub html: Option<String>,
    pub raw_html: Option<String>,
    pub links: Option<Vec<String>>,
    pub metadata: Option<ScrapeMetadata>,
    pub screenshot: Option<String>,
}

/// Firecrawl crawl response
#[derive(Debug, Clone, Deserialize)]
struct FirecrawlCrawlResponse {
    pub success: bool,
    pub id: String,
    pub url: String,
    pub error: Option<String>,
}
