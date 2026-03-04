//! Rhai expression evaluator for strategy conditions and edge/confidence calculations.
use super::DataContext;
use rhai::{Engine, Scope, AST};

/// Compiled Rhai expression ready for evaluation.
pub struct CompiledExpr {
    ast: AST,
}

impl std::fmt::Debug for CompiledExpr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CompiledExpr").finish()
    }
}

/// Create a sandboxed Rhai engine with limited operations.
pub fn create_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_expr_depths(64, 32);
    engine.set_max_operations(10_000);
    engine.set_max_string_size(1024);
    engine.set_max_array_size(256);
    engine.set_max_map_size(128);
    engine
}

impl CompiledExpr {
    /// Compile an expression string. Returns None if compilation fails.
    pub fn compile(engine: &Engine, expr: &str) -> Option<Self> {
        engine.compile_expression(expr).ok().map(|ast| Self { ast })
    }

    /// Evaluate the expression against a DataContext, returning f64.
    pub fn eval_float(&self, engine: &Engine, ctx: &DataContext) -> Option<f64> {
        let mut scope = build_scope(ctx);
        engine
            .eval_ast_with_scope::<f64>(&mut scope, &self.ast)
            .ok()
    }

    /// Evaluate the expression against a DataContext, returning bool.
    pub fn eval_bool(&self, engine: &Engine, ctx: &DataContext) -> Option<bool> {
        let mut scope = build_scope(ctx);
        engine
            .eval_ast_with_scope::<bool>(&mut scope, &self.ast)
            .ok()
    }
}

/// Build a Rhai Scope from a DataContext — all values become variables.
fn build_scope(ctx: &DataContext) -> Scope<'static> {
    let mut scope = Scope::new();
    for (key, val) in &ctx.values {
        match val {
            super::Value::Float(f) => {
                scope.push(key.clone(), *f);
            }
            super::Value::Int(i) => {
                scope.push(key.clone(), *i);
            }
            super::Value::Str(s) => {
                scope.push(key.clone(), s.clone());
            }
            super::Value::Bool(b) => {
                scope.push(key.clone(), *b);
            }
        }
    }
    scope
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strategy_engine::Domain;

    fn test_ctx() -> DataContext {
        let mut ctx = DataContext::new("test", Domain::All);
        ctx.set_float("home_win_rate", 0.72);
        ctx.set_float("market_yes_price", 0.55);
        ctx.set_float("min_edge", 0.08);
        ctx.set_float("home_xg_per90", 2.1);
        ctx
    }

    #[test]
    fn simple_comparison() {
        let engine = create_engine();
        let expr = CompiledExpr::compile(&engine, "home_win_rate > 0.65").unwrap();
        assert_eq!(expr.eval_bool(&engine, &test_ctx()), Some(true));
    }

    #[test]
    fn expression_with_subtraction() {
        let engine = create_engine();
        let expr =
            CompiledExpr::compile(&engine, "home_win_rate - market_yes_price > min_edge").unwrap();
        // 0.72 - 0.55 = 0.17 > 0.08 → true
        assert_eq!(expr.eval_bool(&engine, &test_ctx()), Some(true));
    }

    #[test]
    fn float_expression() {
        let engine = create_engine();
        let expr = CompiledExpr::compile(&engine, "home_win_rate - market_yes_price").unwrap();
        let val = expr.eval_float(&engine, &test_ctx()).unwrap();
        assert!((val - 0.17).abs() < 0.001);
    }

    #[test]
    fn invalid_expr_returns_none() {
        let engine = create_engine();
        assert!(CompiledExpr::compile(&engine, "{{invalid}}").is_none());
    }

    #[test]
    fn clamp_expression() {
        let engine = create_engine();
        let expr = CompiledExpr::compile(
            &engine,
            "if home_win_rate > 1.0 { 1.0 } else { home_win_rate }",
        )
        .unwrap();
        let val = expr.eval_float(&engine, &test_ctx()).unwrap();
        assert!((val - 0.72).abs() < 0.001);
    }
}
