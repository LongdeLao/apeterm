use std::{
    sync::{Arc, Mutex},
    thread,
    time::{Duration, Instant},
};

use reqwest::blocking::Client;
use serde::de::DeserializeOwned;

use crate::config::SecConfig;

#[derive(Clone)]
pub struct SecClient {
    http: Client,
    limiter: Arc<Mutex<TokenBucket>>,
}

#[derive(Debug)]
struct TokenBucket {
    tokens: f64,
    capacity: f64,
    refill_per_second: f64,
    last_refill: Instant,
}

impl SecClient {
    pub fn new(config: &SecConfig) -> Result<Self, String> {
        let http = Client::builder()
            .user_agent(config.user_agent.clone())
            .timeout(Duration::from_secs(20))
            .build()
            .map_err(|error| error.to_string())?;
        let per_second = config.requests_per_second.max(1) as f64;
        Ok(Self {
            http,
            limiter: Arc::new(Mutex::new(TokenBucket {
                tokens: per_second,
                capacity: per_second,
                refill_per_second: per_second,
                last_refill: Instant::now(),
            })),
        })
    }

    pub fn get_json<T: DeserializeOwned>(&self, url: &str) -> Result<T, String> {
        self.request(url)?
            .json::<T>()
            .map_err(|error| error.to_string())
    }

    pub fn get_text(&self, url: &str) -> Result<String, String> {
        self.request(url)?.text().map_err(|error| error.to_string())
    }

    pub fn get_bytes(&self, url: &str) -> Result<Vec<u8>, String> {
        self.request(url)?
            .bytes()
            .map(|bytes| bytes.to_vec())
            .map_err(|error| error.to_string())
    }

    pub fn post_form(
        &self,
        url: &str,
        params: &[(String, String)],
        referer: &str,
    ) -> Result<String, String> {
        let _tokens = self.acquire_token();
        self.http
            .post(url)
            .header(reqwest::header::REFERER, referer)
            .form(params)
            .send()
            .and_then(|response| response.error_for_status())
            .map_err(|error| error.to_string())?
            .text()
            .map_err(|error| error.to_string())
    }

    fn request(&self, url: &str) -> Result<reqwest::blocking::Response, String> {
        let _tokens = self.acquire_token();
        self.http
            .get(url)
            .send()
            .and_then(|response| response.error_for_status())
            .map_err(|error| error.to_string())
    }

    fn acquire_token(&self) -> f64 {
        loop {
            let mut bucket = self.limiter.lock().expect("sec limiter poisoned");
            bucket.refill();
            if bucket.tokens >= 1.0 {
                bucket.tokens -= 1.0;
                return bucket.tokens;
            }
            let wait = ((1.0 - bucket.tokens) / bucket.refill_per_second).max(0.05);
            drop(bucket);
            thread::sleep(Duration::from_secs_f64(wait));
        }
    }
}

impl TokenBucket {
    fn refill(&mut self) {
        let elapsed = self.last_refill.elapsed().as_secs_f64();
        if elapsed <= 0.0 {
            return;
        }
        self.tokens = (self.tokens + elapsed * self.refill_per_second).min(self.capacity);
        self.last_refill = Instant::now();
    }
}
