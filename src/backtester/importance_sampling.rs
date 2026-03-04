//! Importance sampling via exponential tilting for rare-event crash probability estimation.
use rand::rngs::SmallRng;
use rand::SeedableRng;

use super::math::standard_normal;

/// Parameters for importance-sampling crash probability estimation.
pub struct IsParams {
    pub s0: f64,
    /// Crash target as fraction of s0: P(S_T < s0 * k_crash_pct).
    pub k_crash_pct: f64,
    pub sigma: f64,
    pub t_years: f64,
    pub n_paths: usize,
}

pub struct IsResult {
    pub p_is: f64,
    pub se_is: f64,
    pub p_crude: f64,
    pub se_crude: f64,
    /// Variance reduction factor: var_crude / var_is (should be >> 1 for rare events).
    pub variance_reduction: f64,
}

/// Log-return threshold for the crash: ln(k_crash_pct / s0 * s0) = ln(k_crash_pct)
fn log_threshold(params: &IsParams) -> f64 {
    params.k_crash_pct.ln()
}

/// Optimal tilting parameter for exponential tilting toward the crash region.
/// We choose theta so the tilted mean equals the threshold.
/// For GBM log-return X ~ N(mu_x, sigma_x^2):
///   mu_x = -0.5*sigma^2*t, sigma_x = sigma*sqrt(t)
///   Tilted mean: mu_x + theta*sigma_x^2 = log_threshold → theta = (log_threshold - mu_x) / sigma_x^2
fn tilt_param(params: &IsParams) -> f64 {
    let sq = params.sigma * params.sigma * params.t_years;
    let mu_x = -0.5 * sq;
    let threshold = log_threshold(params);
    (threshold - mu_x) / sq
}

/// Simulate crash probability using exponential tilting (importance sampling).
pub fn simulate_is(p: &IsParams) -> IsResult {
    let mut rng = SmallRng::seed_from_u64(42);
    let sigma_t = p.sigma * p.t_years.sqrt();
    let mu_x = -0.5 * p.sigma * p.sigma * p.t_years;
    let threshold = log_threshold(p);
    let theta = tilt_param(p);

    let mut is_payoffs = Vec::with_capacity(p.n_paths);
    let mut crude_payoffs = Vec::with_capacity(p.n_paths);

    for _ in 0..p.n_paths {
        let z = standard_normal(&mut rng);

        // Crude estimate: standard GBM log-return
        let log_ret_crude = mu_x + sigma_t * z;
        crude_payoffs.push(if log_ret_crude < threshold { 1.0 } else { 0.0 });

        // IS estimate: tilted draw (shift z by theta * sigma_t)
        let z_tilt = z + theta * sigma_t;
        let log_ret_tilt = mu_x + sigma_t * z_tilt;
        if log_ret_tilt < threshold {
            // Likelihood ratio = exp(-theta * sigma_t * z - 0.5 * theta^2 * sigma_t^2)
            let lr = (-theta * sigma_t * z - 0.5 * theta * theta * sigma_t * sigma_t).exp();
            is_payoffs.push(lr);
        } else {
            is_payoffs.push(0.0);
        }
    }

    let n = p.n_paths as f64;
    let p_is = is_payoffs.iter().sum::<f64>() / n;
    let var_is = is_payoffs.iter().map(|&x| (x - p_is).powi(2)).sum::<f64>() / (n - 1.0);
    let se_is = (var_is / n).sqrt();

    let p_crude = crude_payoffs.iter().sum::<f64>() / n;
    let var_crude = crude_payoffs
        .iter()
        .map(|&x| (x - p_crude).powi(2))
        .sum::<f64>()
        / (n - 1.0);
    let se_crude = (var_crude / n).sqrt();

    let variance_reduction = if var_is > 0.0 {
        var_crude / var_is
    } else {
        1.0
    };

    IsResult {
        p_is,
        se_is,
        p_crude,
        se_crude,
        variance_reduction,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn crash_params() -> IsParams {
        IsParams {
            s0: 1.0,
            k_crash_pct: 0.3,
            sigma: 0.4,
            t_years: 1.0,
            n_paths: 20_000,
        }
    }

    #[test]
    fn variance_reduction_greater_than_one() {
        let r = simulate_is(&crash_params());
        assert!(r.variance_reduction > 1.0, "VR={}", r.variance_reduction);
    }

    #[test]
    fn crash_probability_in_range() {
        let r = simulate_is(&crash_params());
        assert!(r.p_is >= 0.0, "p_is negative");
        assert!(r.p_is <= 0.1, "p_is too high: {}", r.p_is);
    }

    #[test]
    fn tilt_param_nonzero_for_crash() {
        let p = crash_params();
        let theta = tilt_param(&p);
        assert!(theta < 0.0, "theta should be negative for crash");
    }
}
