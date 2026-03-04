//! Cross-domain strategy registry — scans `strategies/` for *.toml files.
use super::toml_loader::ExprStrategy;
use super::{Domain, UniversalStrategy};
use std::path::Path;

/// Registry of all loaded strategies.
pub struct StrategyRegistry {
    strategies: Vec<Box<dyn UniversalStrategy>>,
}

impl StrategyRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self {
            strategies: Vec::new(),
        }
    }

    /// Scan a directory for *.toml strategy files and load them.
    pub fn load_from_dir(&mut self, dir: &str) {
        let path = Path::new(dir);
        if !path.is_dir() {
            return;
        }
        let entries = match std::fs::read_dir(path) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().map(|e| e == "toml").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&p) {
                    match ExprStrategy::parse(&content) {
                        Ok(strat) => {
                            if strat.manifest().enabled {
                                self.strategies.push(Box::new(strat));
                            }
                        }
                        Err(e) => {
                            eprintln!("Warning: failed to load {}: {}", p.display(), e);
                        }
                    }
                }
            }
        }
    }

    /// Scan multiple directories (e.g. strategies/, strategies/crypto/).
    pub fn load_all_dirs(&mut self, base: &str) {
        self.load_from_dir(base);
        // Also scan subdirectories
        if let Ok(entries) = std::fs::read_dir(base) {
            for entry in entries.flatten() {
                if entry.path().is_dir() {
                    self.load_from_dir(&entry.path().to_string_lossy());
                }
            }
        }
    }

    /// Register a boxed strategy (for built-in Rust strategies).
    pub fn register(&mut self, strat: Box<dyn UniversalStrategy>) {
        self.strategies.push(strat);
    }

    /// Get all strategies.
    pub fn all(&self) -> &[Box<dyn UniversalStrategy>] {
        &self.strategies
    }

    /// Get strategies filtered by domain.
    pub fn by_domain(&self, domain: Domain) -> Vec<&dyn UniversalStrategy> {
        self.strategies
            .iter()
            .filter(|s| s.manifest().domain == domain || s.manifest().domain == Domain::All)
            .map(|s| s.as_ref())
            .collect()
    }

    /// Count of loaded strategies.
    pub fn len(&self) -> usize {
        self.strategies.len()
    }

    pub fn is_empty(&self) -> bool {
        self.strategies.is_empty()
    }

    /// Get strategy names for display.
    pub fn names(&self) -> Vec<(String, String, Domain)> {
        self.strategies
            .iter()
            .map(|s| {
                let m = s.manifest();
                (m.id.clone(), m.name.clone(), m.domain)
            })
            .collect()
    }
}

impl Default for StrategyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy_engine::{DataContext, StrategyManifest};

    struct DummyStrategy {
        manifest: StrategyManifest,
    }

    impl UniversalStrategy for DummyStrategy {
        fn manifest(&self) -> &StrategyManifest {
            &self.manifest
        }
        fn evaluate(&self, _: &[DataContext]) -> Vec<crate::shared::strategy::Signal> {
            Vec::new()
        }
    }

    #[test]
    fn register_and_list() {
        let mut reg = StrategyRegistry::new();
        reg.register(Box::new(DummyStrategy {
            manifest: StrategyManifest {
                id: "test".into(),
                name: "Test".into(),
                domain: Domain::Crypto,
                enabled: true,
                description: String::new(),
                kelly_fraction: 0.25,
                min_edge: 0.05,
                auto_execute: false,
            },
        }));
        assert_eq!(reg.len(), 1);
        assert_eq!(reg.by_domain(Domain::Crypto).len(), 1);
        assert_eq!(reg.by_domain(Domain::Sports).len(), 0);
    }

    #[test]
    fn load_from_nonexistent_dir() {
        let mut reg = StrategyRegistry::new();
        reg.load_from_dir("/nonexistent/path/does/not/exist");
        assert!(reg.is_empty());
    }

    #[test]
    fn names_returns_metadata() {
        let mut reg = StrategyRegistry::new();
        reg.register(Box::new(DummyStrategy {
            manifest: StrategyManifest {
                id: "abc".into(),
                name: "Alpha".into(),
                domain: Domain::Sports,
                enabled: true,
                description: String::new(),
                kelly_fraction: 0.25,
                min_edge: 0.05,
                auto_execute: false,
            },
        }));
        let names = reg.names();
        assert_eq!(names[0].0, "abc");
        assert_eq!(names[0].1, "Alpha");
    }
}
