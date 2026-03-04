//! Sequential Monte Carlo particle filter for tracking hidden event probability.
//! State space: logit(probability); observations: market prices.
use rand::rngs::SmallRng;
use rand::SeedableRng;

use super::math::{logit, sigmoid, standard_normal};

pub struct PfConfig {
    pub n_particles: usize,
    pub prior_prob: f64,
    pub process_vol: f64,
    pub obs_noise: f64,
}

pub struct PfState {
    logit_particles: Vec<f64>,
    weights: Vec<f64>,
    history: Vec<f64>,
}

pub struct PfResult {
    pub final_estimate: f64,
    pub ci_lower: f64,
    pub ci_upper: f64,
    pub ess: f64,
    pub history: Vec<f64>,
}

impl PfState {
    pub fn new(cfg: &PfConfig) -> Self {
        let n = cfg.n_particles;
        let prior_logit = logit(cfg.prior_prob);
        let logit_particles = vec![prior_logit; n];
        let weights = vec![1.0 / n as f64; n];
        Self {
            logit_particles,
            weights,
            history: Vec::new(),
        }
    }

    /// Propagate particles, reweight by observation likelihood, resample if ESS < N/2.
    pub fn update(&mut self, obs: f64, cfg: &PfConfig) {
        let mut rng = SmallRng::seed_from_u64(self.history.len() as u64 + 7);
        let n = self.logit_particles.len();

        // Propagate: x_{t+1} = x_t + process_noise
        for x in &mut self.logit_particles {
            *x += cfg.process_vol * standard_normal(&mut rng);
        }

        // Reweight: likelihood = Gaussian(obs | sigmoid(x), obs_noise)
        let inv_sigma = 1.0 / cfg.obs_noise;
        for (w, &x) in self.weights.iter_mut().zip(self.logit_particles.iter()) {
            let p = sigmoid(x);
            let diff = obs - p;
            let lhood = (-0.5 * diff * diff * inv_sigma * inv_sigma).exp();
            *w *= lhood;
        }

        // Normalize
        let total: f64 = self.weights.iter().sum();
        if total > 0.0 {
            for w in &mut self.weights {
                *w /= total;
            }
        } else {
            let uniform = 1.0 / n as f64;
            self.weights.fill(uniform);
        }

        // Effective sample size
        let ess = {
            let sum_sq: f64 = self.weights.iter().map(|&w| w * w).sum();
            if sum_sq > 0.0 {
                1.0 / sum_sq
            } else {
                1.0
            }
        };

        // Systematic resample if ESS < N/2
        if ess < n as f64 / 2.0 {
            self.systematic_resample();
        }

        self.history.push(self.estimate());
    }

    /// Systematic resampling — O(N), low variance.
    fn systematic_resample(&mut self) {
        use rand::Rng;
        let mut rng = SmallRng::seed_from_u64(999 + self.history.len() as u64);
        let n = self.logit_particles.len();
        let inv_n = 1.0 / n as f64;
        let u0: f64 = rng.gen::<f64>() * inv_n;

        let mut cumulative = 0.0;
        let mut new_particles = Vec::with_capacity(n);
        let mut j = 0usize;
        for i in 0..n {
            let target = u0 + i as f64 * inv_n;
            while j < n - 1 && cumulative + self.weights[j] < target {
                cumulative += self.weights[j];
                j += 1;
            }
            new_particles.push(self.logit_particles[j]);
        }
        self.logit_particles = new_particles;
        self.weights.fill(inv_n);
    }

    pub fn estimate(&self) -> f64 {
        let logit_mean: f64 = self
            .logit_particles
            .iter()
            .zip(self.weights.iter())
            .map(|(&x, &w)| x * w)
            .sum();
        sigmoid(logit_mean)
    }

    pub fn ess(&self) -> f64 {
        let sum_sq: f64 = self.weights.iter().map(|&w| w * w).sum();
        if sum_sq > 0.0 {
            1.0 / sum_sq
        } else {
            1.0
        }
    }

    /// Weighted credible interval at level alpha (e.g. 0.05 → 95% CI).
    pub fn credible_interval(&self, alpha: f64) -> (f64, f64) {
        let mut pairs: Vec<(f64, f64)> = self
            .logit_particles
            .iter()
            .zip(self.weights.iter())
            .map(|(&x, &w)| (sigmoid(x), w))
            .collect();
        pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

        let lo_target = alpha / 2.0;
        let hi_target = 1.0 - alpha / 2.0;
        let mut cum = 0.0;
        let mut lo = pairs[0].0;
        let mut hi = pairs[pairs.len() - 1].0;
        for (p, w) in &pairs {
            cum += w;
            if cum <= lo_target {
                lo = *p;
            }
            if cum >= hi_target {
                hi = *p;
                break;
            }
        }
        (lo, hi)
    }
}

/// Simulate an election-night scenario: 14 price observations trending from 0.50 → 0.95.
pub fn run_election_night(cfg: &PfConfig) -> PfResult {
    let observations: Vec<f64> = (0..14)
        .map(|i| {
            let t = i as f64 / 13.0;
            0.50 + t * 0.45 + (if i % 3 == 0 { -0.02 } else { 0.01 })
        })
        .collect();

    let mut state = PfState::new(cfg);
    for &obs in &observations {
        state.update(obs, cfg);
    }

    let final_estimate = state.estimate();
    let (ci_lower, ci_upper) = state.credible_interval(0.05);
    let ess = state.ess();
    PfResult {
        final_estimate,
        ci_lower,
        ci_upper,
        ess,
        history: state.history.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_cfg() -> PfConfig {
        PfConfig {
            n_particles: 500,
            prior_prob: 0.5,
            process_vol: 0.2,
            obs_noise: 0.05,
        }
    }

    #[test]
    fn new_has_equal_weights() {
        let cfg = default_cfg();
        let pf = PfState::new(&cfg);
        let total: f64 = pf.weights.iter().sum();
        assert!((total - 1.0).abs() < 1e-9, "weights sum to 1");
        assert_eq!(pf.logit_particles.len(), cfg.n_particles);
    }

    #[test]
    fn update_shifts_estimate_toward_obs() {
        let cfg = default_cfg();
        let mut pf = PfState::new(&cfg);
        let before = pf.estimate();
        pf.update(0.9, &cfg);
        let after = pf.estimate();
        assert!(after > before, "estimate should shift toward 0.9 obs");
    }

    #[test]
    fn election_night_converges() {
        let cfg = PfConfig {
            n_particles: 1000,
            prior_prob: 0.5,
            process_vol: 0.15,
            obs_noise: 0.04,
        };
        let result = run_election_night(&cfg);
        assert!(
            result.final_estimate > 0.80,
            "final={}",
            result.final_estimate
        );
        assert_eq!(result.history.len(), 14);
    }

    #[test]
    fn ess_positive_after_update() {
        let cfg = default_cfg();
        let mut pf = PfState::new(&cfg);
        pf.update(0.7, &cfg);
        assert!(pf.ess() > 0.0);
    }
}
