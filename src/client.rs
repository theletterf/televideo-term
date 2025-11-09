use anyhow::{Context, Result};
use regex::Regex;
use scraper::{Html, Selector};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

#[derive(Clone)]
#[allow(dead_code)]
pub struct TelevideoPage {
    pub page_number: u16,
    pub sub_page: u16,
    pub lines: Vec<String>,
    pub timestamp: String,
}

pub struct TelevideoClient {
    cache: HashMap<(u16, u16), (TelevideoPage, SystemTime)>,
    image_cache: HashMap<(u16, u16), (image::DynamicImage, SystemTime)>,
    base_url: String,
    image_base_url: String,
}

impl TelevideoClient {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            image_cache: HashMap::new(),
            base_url: "https://www.servizitelevideo.rai.it/televideo/pub/solotesto.jsp".to_string(),
            image_base_url: "http://www.televideo.rai.it/televideo/pub/tt4web/Nazionale".to_string(),
        }
    }

    pub fn fetch_page(&mut self, page: u16, sub_page: u16) -> Result<TelevideoPage> {
        let cache_key = (page, sub_page);

        // Check cache (5 minute expiry)
        if let Some((data, time)) = self.cache.get(&cache_key) {
            if time.elapsed().unwrap_or(Duration::from_secs(301)) < Duration::from_secs(300) {
                return Ok(data.clone());
            }
        }

        // Build URL for solotesto.jsp
        let url = if sub_page > 1 {
            format!("{}?pagina={}&sottopagina={}", self.base_url, page, sub_page)
        } else {
            format!("{}?pagina={}", self.base_url, page)
        };

        // Fetch the HTML
        let response = reqwest::blocking::get(&url)
            .context("Failed to fetch page")?;

        if !response.status().is_success() {
            anyhow::bail!("Page {}.{} not found", page, sub_page);
        }

        let html = response.text()
            .context("Failed to read response")?;

        // Parse the HTML
        let televideo_page = self.parse_html(&html, page, sub_page)?;

        // Cache it
        self.cache.insert(cache_key, (televideo_page.clone(), SystemTime::now()));

        Ok(televideo_page)
    }

    fn parse_html(&self, html: &str, page: u16, sub_page: u16) -> Result<TelevideoPage> {
        // Extract content between the SOLOTESTO comments
        let start_marker = "<!-- SOLOTESTO PAGINA E SOTTOPAGINA -->";
        let end_marker = "<!-- /SOLOTESTO PAGINA E SOTTOPAGINA -->";

        let content_section = if let Some(start_pos) = html.find(start_marker) {
            let content_start = start_pos + start_marker.len();
            if let Some(end_pos) = html[content_start..].find(end_marker) {
                &html[content_start..content_start + end_pos]
            } else {
                html
            }
        } else {
            html
        };

        let document = Html::parse_fragment(content_section);

        // Find the <pre> tag which contains the formatted content
        let pre_selector = Selector::parse("pre").unwrap();

        let mut lines = Vec::new();

        if let Some(pre_element) = document.select(&pre_selector).next() {
            // Get the HTML content to process links
            let pre_html = pre_element.html();

            // Replace <a> tags with just their text content
            let link_regex = Regex::new(r#"<a href="[^"]*">([^<]+)</a>"#).unwrap();
            let cleaned = link_regex.replace_all(&pre_html, "$1");

            // Parse as HTML to get text with preserved whitespace
            let clean_doc = Html::parse_fragment(&cleaned);
            let text_content = clean_doc.root_element().text().collect::<String>();

            // Split into lines and preserve them
            for line in text_content.lines() {
                lines.push(line.to_string());
            }
        }

        if lines.is_empty() {
            lines.push("(No content found on this page)".to_string());
        }

        Ok(TelevideoPage {
            page_number: page,
            sub_page,
            lines,
            timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
        })
    }

    pub fn fetch_image(&mut self, page: u16, sub_page: u16) -> Result<image::DynamicImage> {
        let cache_key = (page, sub_page);

        // Check cache (5 minute expiry)
        if let Some((img, time)) = self.image_cache.get(&cache_key) {
            if time.elapsed().unwrap_or(Duration::from_secs(301)) < Duration::from_secs(300) {
                return Ok(img.clone());
            }
        }

        // Build URL - use 16:9 widescreen version for better quality
        let url = if sub_page > 1 {
            format!("{}/16_9_page-{}.{}.png", self.image_base_url, page, sub_page)
        } else {
            format!("{}/16_9_page-{}.png", self.image_base_url, page)
        };

        // Fetch the image
        let response = reqwest::blocking::get(&url)
            .context("Failed to fetch image")?;

        if !response.status().is_success() {
            anyhow::bail!("Image for page {}.{} not found", page, sub_page);
        }

        let bytes = response.bytes()
            .context("Failed to read image response")?;

        // Load image from bytes
        let img = image::load_from_memory(&bytes)
            .context("Failed to decode image")?;

        // Cache it
        self.image_cache.insert(cache_key, (img.clone(), SystemTime::now()));

        Ok(img)
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
        self.image_cache.clear();
    }
}
