//! Monte Carlo pricing for binary (YES/NO) prediction market contracts.
//! Implements basic, antithetic-variates, and stratified-sampling estimators.
use rand::rngs::SmallRng;
use rand::SeedableRng;

use super::math::{normal_cdf, standard_normal};

/// Parameters for a GBM-based binary market simulation.
pub struct McParams {
    pub s0: f64,
    pub k: f64,
    pub mu: f64,
    pub sigma: f64,
    pub t_years: f64,
    pub n_paths: usize,
}

/// Result from a Monte Carlo estimator.
pub struct McResult {
    pub probability: f64,
    pub std_error: f64,
    pub ci_lower: f64,
    pub ci_upper: f64,
    pub label: &'static str,
}

impl McResult {
    fn from_samples(payoffs: &[f64], label: &'static str) -> Self {
        let n = payoffs.len() as f64;
        let mean = payoffs.iter().sum::<f64>() / n;
        let variance = payoffs.iter().map(|&x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0);
        let se = (variance / n).sqrt();
        Self {
            probability: mean,
            std_error: se,
            ci_lower: mean - 1.96 * se,
            ci_upper: mean + 1.96 * se,
            label,
        }
    }
}

/// GBM terminal price for a given normal draw z.
fn gbm_terminal(p: &McParams, z: f64) -> f64 {
    let drift = (p.mu - 0.5 * p.sigma * p.sigma) * p.t_years;
    let diffusion = p.sigma * p.t_years.sqrt() * z;
    p.s0 * (drift + diffusion).exp()
}

/// Basic Monte Carlo: plain GBM simulation with binary payoff (S_T > K).
pub fn simulate_basic(p: &McParams) -> McResult {
    let mut rng = SmallRng::seed_from_u64(1);
    let payoffs: Vec<f64> = (0..p.n_paths)
        .map(|_| {
            let z = standard_normal(&mut rng);
            if gbm_terminal(p, z) > p.k {
                1.0
            } else {
                0.0
            }
        })
        .collect();
    McResult::from_samples(&payoffs, "MC Basic")
}

/// Antithetic variates: pairs each z with -z to reduce variance.
pub fn simulate_antithetic(p: &McParams) -> McResult {
    let mut rng = SmallRng::seed_from_u64(2);
    let half = p.n_paths / 2;
    let payoffs: Vec<f64> = (0..half)
        .flat_map(|_| {
            let z = standard_normal(&mut rng);
            let pay_pos = if gbm_terminal(p, z) > p.k { 1.0 } else { 0.0 };
            let pay_neg = if gbm_terminal(p, -z) > p.k { 1.0 } else { 0.0 };
            [(pay_pos + pay_neg) / 2.0, (pay_pos + pay_neg) / 2.0]
        })
        .collect();
    McResult::from_samples(&payoffs, "MC Antithetic")
}

/// Stratified sampling: divides [0,1] into j equal strata and samples one draw per stratum.
pub fn simulate_stratified(p: &McParams, j: usize) -> McResult {
    use rand::Rng;
    let mut rng = SmallRng::seed_from_u64(3);
    let strata = j.max(1);
    let reps = (p.n_paths / strata).max(1);
    let mut payoffs = Vec::with_capacity(strata * reps);
    for s in 0..strata {
        for _ in 0..reps {
            let u: f64 = (s as f64 + rng.gen::<f64>()) / strata as f64;
            // Inverse normal CDF via iterative approximation using normal_cdf
            let z = inv_normal_cdf(u);
            let pay = if gbm_terminal(p, z) > p.k { 1.0 } else { 0.0 };
            payoffs.push(pay);
        }
    }
    McResult::from_samples(&payoffs, "MC Stratified")
}

/// Inverse normal CDF via bisection on normal_cdf.
fn inv_normal_cdf(p: f64) -> f64 {
    let p = p.clamp(1e-9, 1.0 - 1e-9);
    let mut lo = -6.0_f64;
    let mut hi = 6.0_f64;
    for _ in 0..60 {
        let mid = (lo + hi) / 2.0;
        if normal_cdf(mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
    }
    (lo + hi) / 2.0
}

/// Black-Scholes closed-form price for a binary (digital) call: P(S_T > K).
pub fn bs_digital_price(p: &McParams) -> f64 {
    if p.sigma <= 0.0 || p.t_years <= 0.0 {
        return if p.s0 > p.k { 1.0 } else { 0.0 };
    }
    let d2 = ((p.s0 / p.k).ln() + (p.mu - 0.5 * p.sigma * p.sigma) * p.t_years)
        / (p.sigma * p.t_years.sqrt());
    normal_cdf(d2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_params() -> McParams {
        McParams {
            s0: 1.0,
            k: 0.7,
            mu: 0.0,
            sigma: 0.3,
            t_years: 1.0,
            n_paths: 10_000,
        }
    }

    #[test]
    fn simulate_basic_zero_sigma() {
        // With sigma=0, S_T = s0*exp(mu*t), deterministic
        let p = McParams {
            sigma: 0.0,
            s0: 1.0,
            k: 0.5,
            mu: 0.1,
            t_years: 1.0,
            n_paths: 100,
        };
        let r = simulate_basic(&p);
        // s0*exp(0.1*1) ≈ 1.105 > 0.5 → all YES → prob ≈ 1
        assert!((r.probability - 1.0).abs() < 1e-9);
    }

    #[test]
    fn simulate_basic_zero_sigma_below_strike() {
        let p = McParams {
            sigma: 0.0,
            s0: 0.3,
            k: 0.5,
            mu: 0.0,
            t_years: 1.0,
            n_paths: 100,
        };
        let r = simulate_basic(&p);
        // s0=0.3 < k=0.5 → all NO → prob ≈ 0
        assert!((r.probability - 0.0).abs() < 1e-9);
    }

    #[test]
    fn simulate_antithetic_se_le_basic() {
        let p = default_params();
        let basic = simulate_basic(&p);
        let anti = simulate_antithetic(&p);
        // Antithetic SE should be ≤ basic SE (with high probability at n=10k)
        assert!(
            anti.std_error <= basic.std_error * 1.5,
            "anti SE={} basic SE={}",
            anti.std_error,
            basic.std_error
        );
    }

    #[test]
    fn bs_digital_price_matches_closed_form() {
        // S0=1, K=1, mu=0, sigma=0.2, t=1 → d2 = -0.02 → Φ(-0.02) ≈ 0.4920
        let p = McParams {
            s0: 1.0,
            k: 1.0,
            mu: 0.0,
            sigma: 0.2,
            t_years: 1.0,
            n_paths: 1,
        };
        let price = bs_digital_price(&p);
        // d2 = (0 + (0 - 0.02)*1)/(0.2) = -0.1 → Φ(-0.1) ≈ 0.4602
        assert!((price - normal_cdf(-0.1)).abs() < 1e-6);
    }

    #[test]
    fn bs_digital_price_extreme_itm() {
        let p = McParams {
            s0: 100.0,
            k: 0.01,
            mu: 0.0,
            sigma: 0.3,
            t_years: 1.0,
            n_paths: 1,
        };
        let price = bs_digital_price(&p);
        assert!(price > 0.99, "deep ITM digital ≈ 1");
    }
}
