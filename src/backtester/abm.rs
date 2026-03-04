//! Agent-Based Market Model with three agent types and Kyle lambda market impact.
//! Agents: informed (know true_prob), noise (random), market makers (set spread).
use rand::rngs::SmallRng;
use rand::SeedableRng;

use super::math::standard_normal;

pub struct AbmParams {
    pub true_prob: f64,
    pub n_informed: u32,
    pub n_noise: u32,
    pub n_mm: u32,
    pub n_steps: u32,
}

pub struct AbmResult {
    pub final_price: f64,
    pub convergence_error: f64,
    pub volume: f64,
    pub informed_pnl: f64,
    pub price_history: Vec<f64>,
}

struct AbmState {
    price: f64,
    best_bid: f64,
    best_ask: f64,
    volume: f64,
    informed_pnl: f64,
    noise_pnl: f64,
}

impl AbmState {
    fn new(initial_price: f64, half_spread: f64) -> Self {
        Self {
            price: initial_price,
            best_bid: initial_price - half_spread,
            best_ask: initial_price + half_spread,
            volume: 0.0,
            informed_pnl: 0.0,
            noise_pnl: 0.0,
        }
    }
}

/// Kyle lambda: price impact per unit of signed order flow.
/// Higher impact when signal is strong and noise traders are fewer.
pub fn kyle_lambda(true_prob: f64, price: f64, n_noise: u32) -> f64 {
    let signal_strength = (true_prob - price).abs().max(0.01);
    let noise_scale = (n_noise as f64).max(1.0).sqrt();
    signal_strength / noise_scale
}

fn step_informed(state: &mut AbmState, params: &AbmParams, rng: &mut SmallRng) {
    // Informed trader: noisy signal around true_prob, trade toward it
    let signal = params.true_prob + 0.05 * standard_normal(rng);
    let edge = signal - state.price;
    if edge.abs() < 0.01 {
        return; // not enough edge
    }
    let size = edge.abs().min(0.1); // max 10% of bankroll per trade
    let lambda = kyle_lambda(params.true_prob, state.price, params.n_noise);

    if edge > 0.0 {
        // Buy at ask
        let exec_price = state.best_ask;
        let impact = lambda * size;
        state.price = (state.price + impact).min(0.99);
        state.informed_pnl += size * (state.price - exec_price);
    } else {
        // Sell at bid
        let exec_price = state.best_bid;
        let impact = lambda * size;
        state.price = (state.price - impact).max(0.01);
        state.informed_pnl += size * (exec_price - state.price);
    }
    state.volume += size;
}

fn step_noise(state: &mut AbmState, rng: &mut SmallRng) {
    use rand::Rng;
    // Noise trader: random direction, exponential size
    let direction = if rng.gen::<bool>() { 1.0_f64 } else { -1.0_f64 };
    let size = 0.02 * (-rng.gen::<f64>().ln()).min(5.0);
    let impact = 0.005 * size * direction;
    state.price = (state.price + impact).clamp(0.01, 0.99);
    state.volume += size;
    state.noise_pnl -= size * impact.abs(); // noise traders lose on avg
}

fn step_mm(state: &mut AbmState, params: &AbmParams) {
    // Market maker: tighten spread based on volume; update bid/ask around price
    let base_spread = 0.02_f64;
    let volume_factor = (1.0 + state.volume / (params.n_steps as f64 * 0.05)).ln();
    let spread = (base_spread / volume_factor).max(0.005);
    let half = spread / 2.0;
    state.best_bid = (state.price - half).max(0.01);
    state.best_ask = (state.price + half).min(0.99);
}

/// Run the full ABM simulation.
pub fn run(p: &AbmParams) -> AbmResult {
    let mut rng = SmallRng::seed_from_u64(777);
    let initial_price = 0.5_f64;
    let half_spread = 0.01;
    let mut state = AbmState::new(initial_price, half_spread);
    let mut price_history = Vec::with_capacity(p.n_steps as usize);

    for _ in 0..p.n_steps {
        for _ in 0..p.n_informed {
            step_informed(&mut state, p, &mut rng);
        }
        for _ in 0..p.n_noise {
            step_noise(&mut state, &mut rng);
        }
        for _ in 0..p.n_mm {
            step_mm(&mut state, p);
        }
        price_history.push(state.price);
    }

    let convergence_error = (state.price - p.true_prob).abs();
    AbmResult {
        final_price: state.price,
        convergence_error,
        volume: state.volume,
        informed_pnl: state.informed_pnl,
        price_history,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params(true_prob: f64) -> AbmParams {
        AbmParams {
            true_prob,
            n_informed: 5,
            n_noise: 20,
            n_mm: 2,
            n_steps: 2000,
        }
    }

    #[test]
    fn price_converges_up_for_high_true_prob() {
        let r = run(&params(1.0));
        assert!(r.final_price > 0.7, "final_price={}", r.final_price);
    }

    #[test]
    fn price_converges_down_for_low_true_prob() {
        let r = run(&params(0.0));
        assert!(r.final_price < 0.3, "final_price={}", r.final_price);
    }

    #[test]
    fn kyle_lambda_always_positive() {
        for &p in &[0.0, 0.3, 0.5, 0.7, 1.0] {
            for &n in &[1u32, 10, 100] {
                assert!(kyle_lambda(p, 0.5, n) > 0.0);
            }
        }
    }

    #[test]
    fn volume_positive() {
        let r = run(&params(0.6));
        assert!(r.volume > 0.0);
    }

    #[test]
    fn price_history_has_correct_length() {
        let p = params(0.7);
        let r = run(&p);
        assert_eq!(r.price_history.len(), p.n_steps as usize);
    }
}
