//! MmQuoter: compute bid/ask quotes from fair value and half-spread config.

use super::scanner::MmCandidate;
use super::state::{ActiveQuote, QuoteSide};

pub struct MmQuoter {
    pub half_spread: f64,
}

impl MmQuoter {
    pub fn new(half_spread: f64) -> Self {
        Self { half_spread }
    }

    /// Compute fair value as mid-point of best bid/ask.
    pub fn fair_value(candidate: &MmCandidate) -> f64 {
        (candidate.best_bid + candidate.best_ask) / 2.0
    }

    /// Generate bid and ask quotes for a candidate market.
    /// Returns (bid_quote, ask_quote).
    pub fn generate_quotes(
        &self,
        candidate: &MmCandidate,
        size: f64,
    ) -> (ActiveQuote, ActiveQuote) {
        let fair = Self::fair_value(candidate);
        let bid_price = (fair - self.half_spread).clamp(0.01, 0.99);
        let ask_price = (fair + self.half_spread).clamp(0.01, 0.99);

        let bid = ActiveQuote {
            market_id: candidate.market_id.clone(),
            side: QuoteSide::Bid,
            price: bid_price,
            size,
            order_id: format!(
                "paper-bid-{}",
                &candidate.market_id[..8.min(candidate.market_id.len())]
            ),
        };
        let ask = ActiveQuote {
            market_id: candidate.market_id.clone(),
            side: QuoteSide::Ask,
            price: ask_price,
            size,
            order_id: format!(
                "paper-ask-{}",
                &candidate.market_id[..8.min(candidate.market_id.len())]
            ),
        };
        (bid, ask)
    }

    /// Returns true if the fair value has moved enough to warrant a requote.
    pub fn needs_requote(&self, quote: &ActiveQuote, candidate: &MmCandidate) -> bool {
        let fair = Self::fair_value(candidate);
        let expected_price = match quote.side {
            QuoteSide::Bid => fair - self.half_spread,
            QuoteSide::Ask => fair + self.half_spread,
        };
        (quote.price - expected_price).abs() > self.half_spread * 0.5
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_candidate(bid: f64, ask: f64) -> MmCandidate {
        MmCandidate {
            market_id: "abc123def".to_string(),
            title: "Test market".to_string(),
            best_bid: bid,
            best_ask: ask,
            spread: ask - bid,
            volume: 5000.0,
        }
    }

    #[test]
    fn fair_value_is_midpoint() {
        let c = make_candidate(0.44, 0.56);
        assert!((MmQuoter::fair_value(&c) - 0.50).abs() < 0.001);
    }

    #[test]
    fn bid_below_fair_ask_above_fair() {
        let quoter = MmQuoter::new(0.02);
        let c = make_candidate(0.44, 0.56);
        let (bid, ask) = quoter.generate_quotes(&c, 10.0);
        assert!(bid.price < 0.50);
        assert!(ask.price > 0.50);
        assert!((ask.price - bid.price - 0.04).abs() < 0.001);
    }

    #[test]
    fn requote_triggered_when_price_drifts() {
        let quoter = MmQuoter::new(0.02);
        let c = make_candidate(0.44, 0.56);
        let (bid, _) = quoter.generate_quotes(&c, 10.0);
        // Simulate market moved: new mid is now 0.60 → our bid at 0.48 is stale
        let moved = make_candidate(0.58, 0.62);
        assert!(quoter.needs_requote(&bid, &moved));
    }

    #[test]
    fn no_requote_when_price_stable() {
        let quoter = MmQuoter::new(0.02);
        let c = make_candidate(0.44, 0.56);
        let (bid, _) = quoter.generate_quotes(&c, 10.0);
        assert!(!quoter.needs_requote(&bid, &c));
    }
}
