use anyhow::{Context, Result};
use std::collections::HashMap;
use std::time::{Duration, SystemTime};

pub struct TelevideoClient {
    cache: HashMap<(u16, u16), (Vec<u8>, SystemTime)>,
    base_url: String,
}

impl TelevideoClient {
    pub fn new() -> Self {
        Self {
            cache: HashMap::new(),
            base_url: "http://www.televideo.rai.it/televideo/pub/tt4web/Nazionale".to_string(),
        }
    }

    pub fn fetch_page(&mut self, page: u16, part: u16) -> Result<Vec<u8>> {
        let cache_key = (page, part);

        // Check cache (5 minute expiry)
        if let Some((data, time)) = self.cache.get(&cache_key) {
            if time.elapsed().unwrap_or(Duration::from_secs(301)) < Duration::from_secs(300) {
                return Ok(data.clone());
            }
        }

        // Build URL - use 16:9 widescreen version for better quality
        let url = if part > 1 {
            format!("{}/16_9_page-{}.{}.png", self.base_url, page, part)
        } else {
            format!("{}/16_9_page-{}.png", self.base_url, page)
        };

        // Fetch the image
        let response = reqwest::blocking::get(&url)
            .context("Failed to fetch page")?;

        if !response.status().is_success() {
            anyhow::bail!("Page {}.{} not found", page, part);
        }

        let bytes = response.bytes()
            .context("Failed to read response")?
            .to_vec();

        // Cache it
        self.cache.insert(cache_key, (bytes.clone(), SystemTime::now()));

        Ok(bytes)
    }

    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }
}
