//! Brier score calibration from paper-trade JSONL logs.
use std::fs;
use std::io::{self, BufRead};

pub struct CalibRecord {
    pub predicted: f64,
    pub outcome: Option<bool>,
    pub strategy_id: String,
}

pub struct CalibResult {
    pub brier_score: Option<f64>,
    pub n_resolved: usize,
    pub n_pending: usize,
    pub accuracy: Option<f64>,
}

/// Load calibration records from a paper-trades JSONL file.
/// Lines that cannot be parsed are silently skipped.
pub fn load_from_jsonl(path: &str) -> Vec<CalibRecord> {
    let file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return Vec::new(),
    };
    let reader = io::BufReader::new(file);
    let mut records = Vec::new();

    for line in reader.lines().map_while(Result::ok) {
        let trimmed = line.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        let Ok(val) = serde_json::from_str::<serde_json::Value>(&trimmed) else {
            continue;
        };
        let predicted = match val.get("predicted").or_else(|| val.get("confidence")) {
            Some(v) => match v.as_f64() {
                Some(f) => f,
                None => continue,
            },
            None => continue,
        };
        let outcome = val.get("outcome").and_then(|o| o.as_bool()).or_else(|| {
            val.get("resolved")
                .and_then(|r| r.as_str())
                .map(|s| s == "YES" || s == "yes")
        });
        let strategy_id = val
            .get("strategy_id")
            .and_then(|s| s.as_str())
            .unwrap_or("unknown")
            .to_string();
        records.push(CalibRecord {
            predicted,
            outcome,
            strategy_id,
        });
    }
    records
}

/// Brier score = mean((predicted - outcome)^2) over resolved records.
pub fn brier_score(records: &[CalibRecord]) -> Option<f64> {
    let resolved: Vec<_> = records.iter().filter(|r| r.outcome.is_some()).collect();
    if resolved.is_empty() {
        return None;
    }
    let score: f64 = resolved
        .iter()
        .map(|r| {
            let o = if r.outcome.unwrap_or(false) { 1.0 } else { 0.0 };
            (r.predicted - o).powi(2)
        })
        .sum::<f64>()
        / resolved.len() as f64;
    Some(score)
}

impl CalibResult {
    pub fn compute(records: &[CalibRecord]) -> Self {
        let resolved: Vec<_> = records.iter().filter(|r| r.outcome.is_some()).collect();
        let n_resolved = resolved.len();
        let n_pending = records.len() - n_resolved;

        let brier = brier_score(records);
        let accuracy = if n_resolved == 0 {
            None
        } else {
            let correct = resolved
                .iter()
                .filter(|r| {
                    let pred_yes = r.predicted >= 0.5;
                    let outcome_yes = r.outcome.unwrap_or(false);
                    pred_yes == outcome_yes
                })
                .count();
            Some(correct as f64 / n_resolved as f64)
        };

        Self {
            brier_score: brier,
            n_resolved,
            n_pending,
            accuracy,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn perfect_records() -> Vec<CalibRecord> {
        vec![
            CalibRecord {
                predicted: 1.0,
                outcome: Some(true),
                strategy_id: "s1".into(),
            },
            CalibRecord {
                predicted: 0.0,
                outcome: Some(false),
                strategy_id: "s1".into(),
            },
        ]
    }

    fn worst_records() -> Vec<CalibRecord> {
        vec![
            CalibRecord {
                predicted: 0.0,
                outcome: Some(true),
                strategy_id: "s1".into(),
            },
            CalibRecord {
                predicted: 1.0,
                outcome: Some(false),
                strategy_id: "s1".into(),
            },
        ]
    }

    #[test]
    fn brier_perfect_predictions() {
        let r = brier_score(&perfect_records());
        assert_eq!(r, Some(0.0));
    }

    #[test]
    fn brier_worst_predictions() {
        let r = brier_score(&worst_records());
        assert_eq!(r, Some(1.0));
    }

    #[test]
    fn load_from_missing_file_returns_empty() {
        let r = load_from_jsonl("/nonexistent/path/paper_trades.jsonl");
        assert!(r.is_empty());
    }

    #[test]
    fn calib_result_no_resolved_records() {
        let records = vec![CalibRecord {
            predicted: 0.7,
            outcome: None,
            strategy_id: "x".into(),
        }];
        let result = CalibResult::compute(&records);
        assert!(result.brier_score.is_none());
        assert_eq!(result.n_pending, 1);
        assert_eq!(result.n_resolved, 0);
    }

    #[test]
    fn calib_result_accuracy_computed() {
        let result = CalibResult::compute(&perfect_records());
        assert_eq!(result.accuracy, Some(1.0));
        assert_eq!(result.n_resolved, 2);
    }
}
