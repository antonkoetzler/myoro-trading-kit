//! Pure mathematical primitives for simulation modules.
use rand::Rng;

/// Polar Box-Muller transform — generates standard normal samples.
pub fn standard_normal<R: Rng>(rng: &mut R) -> f64 {
    loop {
        let u = rng.gen::<f64>() * 2.0 - 1.0;
        let v = rng.gen::<f64>() * 2.0 - 1.0;
        let s = u * u + v * v;
        if s > 0.0 && s < 1.0 {
            return u * (-2.0 * s.ln() / s).sqrt();
        }
    }
}

/// Normal CDF via Abramowitz & Stegun rational approximation (max error < 7.5e-8).
pub fn normal_cdf(x: f64) -> f64 {
    const A1: f64 = 0.319_381_530;
    const A2: f64 = -0.356_563_782;
    const A3: f64 = 1.781_477_937;
    const A4: f64 = -1.821_255_978;
    const A5: f64 = 1.330_274_429;
    let sign = if x < 0.0 { -1.0 } else { 1.0 };
    let x = x.abs();
    let t = 1.0 / (1.0 + 0.231_641_9 * x);
    let pdf = (-0.5 * x * x).exp() / (2.0 * std::f64::consts::PI).sqrt();
    let poly = t * (A1 + t * (A2 + t * (A3 + t * (A4 + t * A5))));
    let cdf_pos = 1.0 - pdf * poly;
    0.5 + sign * (cdf_pos - 0.5)
}

/// Logit (log-odds), clamped to avoid infinities.
pub fn logit(p: f64) -> f64 {
    let p = p.clamp(1e-9, 1.0 - 1e-9);
    (p / (1.0 - p)).ln()
}

/// Sigmoid (inverse logit).
pub fn sigmoid(x: f64) -> f64 {
    1.0 / (1.0 + (-x).exp())
}

/// Cholesky L for 2×2 correlation matrix [[1,ρ],[ρ,1]].
/// Returns L such that L * L^T = corr_matrix.
pub fn cholesky_2x2(corr: f64) -> [[f64; 2]; 2] {
    [[1.0, 0.0], [corr, (1.0 - corr * corr).sqrt()]]
}

/// Lower-triangular Cholesky decomposition for an n×n PD matrix.
pub fn cholesky_nxn(m: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = m.len();
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let sum: f64 = (0..j).map(|k| l[i][k] * l[j][k]).sum();
            if i == j {
                l[i][j] = (m[i][i] - sum).sqrt();
            } else {
                l[i][j] = (m[i][j] - sum) / l[j][j];
            }
        }
    }
    l
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::rngs::SmallRng;
    use rand::SeedableRng;

    #[test]
    fn standard_normal_bounds_and_mean() {
        let mut rng = SmallRng::seed_from_u64(42);
        let samples: Vec<f64> = (0..10_000).map(|_| standard_normal(&mut rng)).collect();
        let mean = samples.iter().sum::<f64>() / samples.len() as f64;
        assert!(samples.iter().all(|&x| x.abs() < 6.0), "all within ±6σ");
        assert!(mean.abs() < 0.1, "mean ≈ 0, got {mean}");
    }

    #[test]
    fn normal_cdf_known_values() {
        assert!((normal_cdf(0.0) - 0.5).abs() < 1e-6, "CDF(0) ≈ 0.5");
        assert!((normal_cdf(1.645) - 0.95).abs() < 1e-3, "CDF(1.645) ≈ 0.95");
        assert!(normal_cdf(-4.0) < 1e-4, "CDF(-4) < 1e-4");
        assert!(normal_cdf(4.0) > 0.9999, "CDF(4) > 0.9999");
    }

    #[test]
    fn logit_sigmoid_roundtrip() {
        assert!(logit(0.5).abs() < 1e-12, "logit(0.5) == 0");
        let p = 0.3_f64;
        assert!(
            (sigmoid(logit(p)) - p).abs() < 1e-9,
            "sigmoid(logit(p)) ≈ p"
        );
    }

    #[test]
    fn cholesky_2x2_reconstructs() {
        let rho = 0.6_f64;
        let l = cholesky_2x2(rho);
        // L * L^T should equal [[1, rho], [rho, 1]]
        let ll00 = l[0][0] * l[0][0] + l[0][1] * l[0][1];
        let ll01 = l[0][0] * l[1][0] + l[0][1] * l[1][1];
        let ll11 = l[1][0] * l[1][0] + l[1][1] * l[1][1];
        assert!((ll00 - 1.0).abs() < 1e-12);
        assert!((ll01 - rho).abs() < 1e-12);
        assert!((ll11 - 1.0).abs() < 1e-12);
    }

    #[test]
    fn cholesky_nxn_identity() {
        let identity = vec![
            vec![1.0, 0.0, 0.0],
            vec![0.0, 1.0, 0.0],
            vec![0.0, 0.0, 1.0],
        ];
        let l = cholesky_nxn(&identity);
        for i in 0..3 {
            for j in 0..3 {
                let expected = if i == j { 1.0 } else { 0.0 };
                assert!((l[i][j] - expected).abs() < 1e-12, "L[{i}][{j}] off");
            }
        }
    }
}
