use crate::config::Config;
use crate::ibkr_client::HistoricalMarketDataEntry;
use crate::stock_candidates::StockCandidates;
use crate::stock_data_downloader::ContractId;
use crate::stock_data_downloader::StockData;
use crate::stock_ranker::Ticker;
use chrono::DateTime;
use chrono::Utc;
use serde::Deserialize;
use std::rc::Rc;

pub struct ScoringFactorExtractor {
    config: Rc<Config>,
}

impl ScoringFactorExtractor {
    pub fn new(config: Rc<Config>) -> Self {
        Self { config }
    }

    pub fn extract_scoring_factors(&self, stock_data: &StockData) -> StockCandidates {
        let mut candidates = StockCandidates::from_config_overrides(&self.config.r#override);
        for position in &stock_data.portfolio {
            let conid = position.conid.into();
            let ticker: Ticker = position.ticker.as_str().into();

            // Extract P/E
            if let Some(notional) = stock_data
                .market_snapshot
                .get(&conid)
                .and_then(|snapshot| snapshot.pe_ratio)
            {
                candidates.add_candidate(ticker.clone(), ScoringFactor::PeRatio, notional.into());
            }

            // Long-term price change
            if let Some(notional) = extract_long_term_price_change(conid, stock_data) {
                candidates.add_candidate(
                    ticker.clone(),
                    ScoringFactor::LongTermChange,
                    notional.into(),
                )
            }

            // Short-term price change
            if let Some(notional) = extract_short_term_price_change(conid, stock_data) {
                candidates.add_candidate(
                    ticker.clone(),
                    ScoringFactor::ShortTermChange,
                    notional.into(),
                )
            }
        }
        candidates
    }
}

#[derive(Hash, PartialEq, Eq, Clone, Copy, Debug, Deserialize)]
pub enum ScoringFactor {
    /// Price over earnings.
    PeRatio,

    /// Change of the stock price in the long term.
    LongTermChange,

    /// Change of the stock price in the short term.
    ShortTermChange,
}

fn extract_long_term_price_change(conid: ContractId, stock_data: &StockData) -> Option<f64> {
    let last_price = stock_data.market_snapshot.get(&conid)?.last_price?;
    let oldest_market_data = stock_data.long_term_market_history.get(&conid)?.first()?;
    let five_years = 1000 * 60 * 60 * 24 * 365 * 5;
    let now = Utc::now().timestamp_millis();
    if now - oldest_market_data.t < five_years {
        None
    } else {
        price_change(oldest_market_data.c, last_price)
    }
}

fn extract_short_term_price_change(conid: ContractId, stock_data: &StockData) -> Option<f64> {
    let last_price = stock_data.market_snapshot.get(&conid)?.last_price?;
    let history = stock_data.short_term_market_history.get(&conid)?;
    let price_on_last_month = last_month_entry(history)?.c;
    price_change(price_on_last_month, last_price)
}

fn price_change(old_price: f64, new_price: f64) -> Option<f64> {
    if old_price == 0.0 {
        None
    } else {
        Some((new_price - old_price) / old_price)
    }
}

fn last_month_entry(history: &[HistoricalMarketDataEntry]) -> Option<&HistoricalMarketDataEntry> {
    let now = Utc::now();
    history
        .iter()
        .rev()
        .find(|entry| within_1_month(entry.t, now))
}

fn within_1_month(timestamp: i64, now: DateTime<Utc>) -> bool {
    let duration = now.timestamp_millis() - timestamp;
    let one_month = 1000 * 60 * 60 * 24 * 30;
    one_month <= duration && duration <= one_month * 2
}

#[cfg(test)]
mod test {
    use super::*;
    use chrono::Duration;

    #[test]
    fn price_change() {
        // Positive change
        let change = super::price_change(100.0, 200.0);
        assert_eq!(Some(1.0), change);

        // Negative change
        let change = super::price_change(200.0, 100.0);
        assert_eq!(Some(-0.5), change);

        // Division by 0
        let change = super::price_change(0.0, 100.0);
        assert_eq!(None, change);
    }

    #[test]
    fn last_month_entry() {
        // Test case
        let history = [
            HistoricalMarketDataEntry {
                c: 1.0,
                t: (Utc::now() - Duration::days(300)).timestamp_millis(),
            },
            HistoricalMarketDataEntry {
                c: 2.0,
                t: (Utc::now() - Duration::days(200)).timestamp_millis(),
            },
            HistoricalMarketDataEntry {
                c: 3.0,
                t: (Utc::now() - Duration::days(100)).timestamp_millis(),
            },
        ];
        let found_entry = super::last_month_entry(&history);
        assert_eq!(None, found_entry);

        // Test case
        let history = [
            HistoricalMarketDataEntry {
                c: 1.0,
                t: (Utc::now() - Duration::days(5)).timestamp_millis(),
            },
            HistoricalMarketDataEntry {
                c: 2.0,
                t: (Utc::now() - Duration::days(4)).timestamp_millis(),
            },
            HistoricalMarketDataEntry {
                c: 3.0,
                t: (Utc::now() - Duration::days(3)).timestamp_millis(),
            },
        ];
        let found_entry = super::last_month_entry(&history);
        assert_eq!(None, found_entry);

        // Test case
        let history = [
            HistoricalMarketDataEntry {
                c: 1.0,
                t: (Utc::now() - Duration::days(100)).timestamp_millis(),
            },
            HistoricalMarketDataEntry {
                c: 2.0,
                t: (Utc::now() - Duration::days(35)).timestamp_millis(),
            },
            HistoricalMarketDataEntry {
                c: 3.0,
                t: (Utc::now() - Duration::days(30)).timestamp_millis(),
            },
            HistoricalMarketDataEntry {
                c: 4.0,
                t: (Utc::now() - Duration::days(10)).timestamp_millis(),
            },
        ];
        let found_entry = super::last_month_entry(&history);
        assert_eq!(3.0, found_entry.unwrap().c);
    }
}
