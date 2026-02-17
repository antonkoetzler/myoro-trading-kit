//! News stub: trait + no-op or RSS impl for later paid feed.

use anyhow::Result;

pub trait NewsFeed: Send + Sync {
    fn poll(&self) -> Result<Vec<NewsItem>>;
}

#[derive(Clone, Debug)]
pub struct NewsItem {
    pub headline: String,
    pub source: String,
    pub ts: u64,
}

pub struct NoopNewsFeed;

impl NewsFeed for NoopNewsFeed {
    fn poll(&self) -> Result<Vec<NewsItem>> {
        Ok(Vec::new())
    }
}
