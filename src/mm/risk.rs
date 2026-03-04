//! MmRisk: per-market inventory limits and total exposure guard.

use crate::config::Config;
use anyhow::{bail, Result};

pub struct MmRisk {
    pub max_inventory_usd: f64,
    pub max_total_exposure: f64,
}

impl MmRisk {
    pub fn from_config(config: &Config) -> Self {
        Self {
            max_inventory_usd: config.mm_max_inventory_usd,
            max_total_exposure: config.mm_max_inventory_usd * config.mm_max_markets as f64,
        }
    }

    /// Check if posting a new quote would violate inventory limits.
    /// `current_inventory` = abs net position for this market.
    /// `total_exposure` = sum of abs net positions across all markets.
    pub fn check_quote(
        &self,
        current_inventory: f64,
        total_exposure: f64,
        quote_size: f64,
    ) -> Result<()> {
        if current_inventory + quote_size > self.max_inventory_usd {
            bail!(
                "Market inventory limit reached: {:.2} + {:.2} > {:.2}",
                current_inventory,
                quote_size,
                self.max_inventory_usd
            );
        }
        if total_exposure + quote_size > self.max_total_exposure {
            bail!(
                "Total exposure limit reached: {:.2} + {:.2} > {:.2}",
                total_exposure,
                quote_size,
                self.max_total_exposure
            );
        }
        Ok(())
    }

    /// Returns true if this side of the market is at inventory limit.
    pub fn at_limit(&self, current_inventory: f64) -> bool {
        current_inventory >= self.max_inventory_usd
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn test_config() -> Config {
        Config {
            mm_max_inventory_usd: 200.0,
            mm_max_markets: 5,
            ..Config::default()
        }
    }

    #[test]
    fn check_quote_passes_within_limits() {
        let risk = MmRisk::from_config(&test_config());
        assert!(risk.check_quote(50.0, 200.0, 10.0).is_ok());
    }

    #[test]
    fn check_quote_fails_at_market_limit() {
        let risk = MmRisk::from_config(&test_config());
        assert!(risk.check_quote(195.0, 200.0, 10.0).is_err());
    }

    #[test]
    fn check_quote_fails_at_total_exposure_limit() {
        let risk = MmRisk::from_config(&test_config());
        // max_total = 200 * 5 = 1000
        assert!(risk.check_quote(10.0, 995.0, 10.0).is_err());
    }

    #[test]
    fn at_limit_returns_true_when_full() {
        let risk = MmRisk::from_config(&test_config());
        assert!(risk.at_limit(200.0));
        assert!(!risk.at_limit(199.0));
    }
}
