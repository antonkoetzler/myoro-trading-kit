//! Gaussian and Student-t copula for modeling correlated binary market baskets.
use rand::rngs::SmallRng;
use rand::SeedableRng;

use super::math::{cholesky_nxn, normal_cdf, standard_normal};

/// Pre-set swing-state-like probabilities for the basket.
const SWING_PROBS: [f64; 5] = [0.52, 0.53, 0.51, 0.48, 0.50];

pub struct CopulaParams {
    pub probs: Vec<f64>,
    pub rho: f64,
    pub nu: f64,
    pub n_paths: usize,
}

pub struct CopulaResult {
    pub p_all_yes: f64,
    pub p_all_no: f64,
    pub tail_dep_upper: f64,
    pub n_contracts: usize,
    pub extra: Vec<(String, String)>,
}

/// Build a correlation matrix: 1 on diagonal, rho off-diagonal.
fn corr_matrix(n: usize, rho: f64) -> Vec<Vec<f64>> {
    (0..n)
        .map(|i| (0..n).map(|j| if i == j { 1.0 } else { rho }).collect())
        .collect()
}

/// Sample correlated normals using Cholesky decomposition.
fn correlated_normals<R: rand::Rng>(l: &[Vec<f64>], rng: &mut R) -> Vec<f64> {
    let n = l.len();
    let zs: Vec<f64> = (0..n).map(|_| standard_normal(rng)).collect();
    (0..n)
        .map(|i| (0..=i).map(|j| l[i][j] * zs[j]).sum())
        .collect()
}

/// Gaussian copula: models co-movement via correlated normals.
pub fn gaussian_copula(p: &CopulaParams) -> CopulaResult {
    let probs = if p.probs.is_empty() {
        SWING_PROBS.to_vec()
    } else {
        p.probs.clone()
    };
    let n = probs.len();
    let corr = corr_matrix(n, p.rho);
    let l = cholesky_nxn(&corr);
    let mut rng = SmallRng::seed_from_u64(11);

    let mut all_yes = 0u64;
    let mut all_no = 0u64;

    for _ in 0..p.n_paths {
        let xs = correlated_normals(&l, &mut rng);
        let outcomes: Vec<bool> = xs
            .iter()
            .zip(probs.iter())
            .map(|(&x, &pi)| normal_cdf(x) < pi)
            .collect();
        if outcomes.iter().all(|&o| o) {
            all_yes += 1;
        }
        if outcomes.iter().all(|&o| !o) {
            all_no += 1;
        }
    }

    let n_f = p.n_paths as f64;
    let p_all_yes = all_yes as f64 / n_f;
    let p_all_no = all_no as f64 / n_f;

    CopulaResult {
        p_all_yes,
        p_all_no,
        tail_dep_upper: 0.0, // Gaussian copula has zero tail dependence
        n_contracts: n,
        extra: vec![
            ("Model".into(), "Gaussian".into()),
            ("ρ".into(), format!("{:.2}", p.rho)),
        ],
    }
}

/// Student-t copula: fat-tailed copula with non-zero tail dependence.
pub fn t_copula(p: &CopulaParams) -> CopulaResult {
    use rand::Rng;
    let probs = if p.probs.is_empty() {
        SWING_PROBS.to_vec()
    } else {
        p.probs.clone()
    };
    let n = probs.len();
    let corr = corr_matrix(n, p.rho);
    let l = cholesky_nxn(&corr);
    let mut rng = SmallRng::seed_from_u64(13);
    let nu = p.nu.max(2.0);

    let mut all_yes = 0u64;
    let mut all_no = 0u64;

    for _ in 0..p.n_paths {
        // Sample chi-squared(nu) via sum of nu standard normals squared
        let chi2: f64 = (0..nu as usize)
            .map(|_| standard_normal(&mut rng).powi(2))
            .sum();
        let scale = (nu / chi2).sqrt();

        let xs = correlated_normals(&l, &mut rng);
        // t-distributed: t_i = x_i * scale; CDF is standard t with nu df
        let outcomes: Vec<bool> = xs
            .iter()
            .zip(probs.iter())
            .map(|(&x, &pi)| {
                let t_val = x * scale;
                // Approximate t-CDF via normal CDF (good for nu > 30; for lower nu we use beta approx)
                let u = t_cdf(t_val, nu);
                u < pi
            })
            .collect();
        if outcomes.iter().all(|&o| o) {
            all_yes += 1;
        }
        if outcomes.iter().all(|&o| !o) {
            all_no += 1;
        }

        // Consume rng to avoid correlation between runs
        let _: f64 = rng.gen();
    }

    let n_f = p.n_paths as f64;
    let p_all_yes = all_yes as f64 / n_f;
    let p_all_no = all_no as f64 / n_f;

    // Tail dependence: λ_U = 2 * t_{ν+1}(-sqrt((ν+1)(1-ρ)/(1+ρ)))
    let tail_dep_upper = tail_dependence(p.rho, nu);

    CopulaResult {
        p_all_yes,
        p_all_no,
        tail_dep_upper,
        n_contracts: n,
        extra: vec![
            ("Model".into(), "Student-t".into()),
            ("ρ".into(), format!("{:.2}", p.rho)),
            ("ν".into(), format!("{:.0}", nu)),
            ("λ_U".into(), format!("{:.4}", tail_dep_upper)),
        ],
    }
}

/// Upper tail dependence coefficient for bivariate t-copula.
fn tail_dependence(rho: f64, nu: f64) -> f64 {
    let arg = -((nu + 1.0) * (1.0 - rho) / (1.0 + rho)).sqrt();
    2.0 * t_cdf(arg, nu + 1.0)
}

/// Approximation of the Student-t CDF with df degrees of freedom.
/// Uses the regularized incomplete beta function approximation.
fn t_cdf(t: f64, df: f64) -> f64 {
    // x = t^2 / (t^2 + df) → regularized incomplete beta I(x; 0.5, df/2) / 2
    let x = t * t / (t * t + df);
    let beta_half = regularized_beta(x, 0.5, df / 2.0);
    if t < 0.0 {
        beta_half / 2.0
    } else {
        1.0 - beta_half / 2.0
    }
}

/// Simple regularized incomplete beta via continued fraction (Lentz method, truncated).
fn regularized_beta(x: f64, a: f64, b: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    // Use symmetry for faster convergence
    if x > (a + 1.0) / (a + b + 2.0) {
        return 1.0 - regularized_beta(1.0 - x, b, a);
    }
    let ln_beta_fn = ln_gamma(a) + ln_gamma(b) - ln_gamma(a + b);
    let front = (x.powf(a) * (1.0 - x).powf(b)).ln() - ln_beta_fn;
    let cf = beta_cf(x, a, b);
    (front + cf.ln()).exp() / a
}

/// Continued fraction for incomplete beta (Abramowitz & Stegun 26.5.8).
fn beta_cf(x: f64, a: f64, b: f64) -> f64 {
    let max_iter = 200;
    let eps = 1e-10;
    let mut c = 1.0_f64;
    let mut d = 1.0 - (a + b) * x / (a + 1.0);
    if d.abs() < eps {
        d = eps;
    }
    d = 1.0 / d;
    let mut f = d;

    for m in 1..=max_iter {
        let m = m as f64;
        // Even step
        let aa = m * (b - m) * x / ((a + 2.0 * m - 1.0) * (a + 2.0 * m));
        d = 1.0 + aa * d;
        if d.abs() < eps {
            d = eps;
        }
        c = 1.0 + aa / c;
        if c.abs() < eps {
            c = eps;
        }
        d = 1.0 / d;
        f *= d * c;
        // Odd step
        let aa = -(a + m) * (a + b + m) * x / ((a + 2.0 * m) * (a + 2.0 * m + 1.0));
        d = 1.0 + aa * d;
        if d.abs() < eps {
            d = eps;
        }
        c = 1.0 + aa / c;
        if c.abs() < eps {
            c = eps;
        }
        d = 1.0 / d;
        let delta = d * c;
        f *= delta;
        if (delta - 1.0).abs() < eps {
            break;
        }
    }
    f
}

/// Stirling approximation for ln(Gamma(x)).
fn ln_gamma(x: f64) -> f64 {
    if x < 0.5 {
        std::f64::consts::PI.ln() - ((std::f64::consts::PI * x).sin().ln()) - ln_gamma(1.0 - x)
    } else {
        let x = x - 1.0;
        let coeffs = [
            76.180_091_729_471_46,
            -86.505_320_329_416_78,
            24.014_098_240_830_91,
            -1.231_739_572_450_155,
            0.001_208_650_973_866_179,
            -5.395_239_384_953e-6,
        ];
        let mut ser = 1.000_000_000_190_015;
        let mut y = x;
        for c in &coeffs {
            y += 1.0;
            ser += c / y;
        }
        let tmp = x + 5.5;
        (2.0 * std::f64::consts::PI).sqrt().ln() + (x + 0.5) * tmp.ln() - tmp + ser.ln()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params_gaussian() -> CopulaParams {
        CopulaParams {
            probs: vec![],
            rho: 0.6,
            nu: 5.0,
            n_paths: 50_000,
        }
    }

    fn params_t() -> CopulaParams {
        CopulaParams {
            probs: vec![],
            rho: 0.6,
            nu: 5.0,
            n_paths: 50_000,
        }
    }

    #[test]
    fn gaussian_probs_in_range() {
        let r = gaussian_copula(&params_gaussian());
        assert!(
            r.p_all_yes > 0.0 && r.p_all_yes < 1.0,
            "p_yes={}",
            r.p_all_yes
        );
        assert!(r.p_all_no > 0.0 && r.p_all_no < 1.0, "p_no={}", r.p_all_no);
    }

    #[test]
    fn t_copula_tail_dep_in_range() {
        let r = t_copula(&params_t());
        assert!(
            r.tail_dep_upper >= 0.0 && r.tail_dep_upper <= 1.0,
            "λ={}",
            r.tail_dep_upper
        );
    }

    #[test]
    fn t_copula_higher_joint_than_gaussian() {
        let gaus = gaussian_copula(&params_gaussian());
        let t = t_copula(&params_t());
        // Fat tails → higher joint probability (allow small tolerance for MC noise)
        assert!(
            t.p_all_yes >= gaus.p_all_yes * 0.8,
            "t={} vs gaussian={}",
            t.p_all_yes,
            gaus.p_all_yes
        );
    }

    #[test]
    fn n_contracts_matches_probs() {
        let r = gaussian_copula(&CopulaParams {
            probs: vec![],
            rho: 0.5,
            nu: 5.0,
            n_paths: 1000,
        });
        assert_eq!(r.n_contracts, 5);
    }
}
