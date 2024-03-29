use crate::stock_data_downloader::StockData;
use crate::stock_data_downloader::StockDataDownloader;
use anyhow::Context;
use chrono::DateTime;
use chrono::Utc;
use std::path::PathBuf;

pub struct StockDataCacher {
    downloader: StockDataDownloader,
    cache_path: PathBuf,
}

impl Default for StockDataCacher {
    fn default() -> Self {
        let mut cache_path = std::env::temp_dir();
        cache_path.push("ibkr-toy-cache.json");
        Self {
            downloader: StockDataDownloader::default(),
            cache_path,
        }
    }
}

impl StockDataCacher {
    pub async fn fetch(&self, account_id: &str, use_cache: bool) -> anyhow::Result<StockData> {
        if !use_cache {
            println!("Downloading stock data")
        } else if let Ok(stock_data) = self.read_cache().await {
            if !cache_outdated(stock_data.timestamp) {
                println!("Generating report using cached data");
                return Ok(stock_data);
            } else {
                println!("Cache is outdated");
            }
        } else {
            println!("Stock data not found in cache");
        }

        println!("Downloading stock data from IBKR");
        let stock_data = self
            .downloader
            .download_stock_data(account_id)
            .await
            .context("Failed to download stock data")?;

        let stock_data_serialized =
            serde_json::to_string(&stock_data).context("Failed to serialize stock data to JSON")?;
        tokio::fs::write(&self.cache_path, stock_data_serialized)
            .await
            .context("Failed to write cache")?;

        Ok(stock_data)
    }

    async fn read_cache(&self) -> anyhow::Result<StockData> {
        let cache = tokio::fs::read_to_string(&self.cache_path).await?;
        let stock_data = serde_json::from_str(&cache)?;
        Ok(stock_data)
    }
}

fn cache_outdated(timstamp: DateTime<Utc>) -> bool {
    (Utc::now() - timstamp).num_days() >= 1
}
