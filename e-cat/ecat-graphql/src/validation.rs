// Copyright (c) 2026 erik <erik@erik.xyz> — https://erik.xyz
//! 执行前查询预算校验：防查询放大 DoS。
//! 解析器已限制 MAX_DEPTH=32 并拒绝别名/多顶层字段，此处补节点总数预算。

use crate::parser::{ParsedField, SelectionSet};

pub const DEFAULT_MAX_COMPLEXITY: usize = 10_000;

#[derive(Debug, Clone, Copy)]
pub struct QueryLimits {
    /// 选择树节点总数上限（含根字段），超过即拒绝。
    pub max_complexity: usize,
}

impl Default for QueryLimits {
    fn default() -> Self {
        Self {
            max_complexity: DEFAULT_MAX_COMPLEXITY,
        }
    }
}

/// 解析通过后、任何执行工作开始前调用。超过预算返回错误字符串
/// （与 parse 错误同一形状，由 execute 打包为 GraphQL errors）。
pub fn validate(field: &ParsedField, limits: &QueryLimits) -> Result<(), String> {
    let n = complexity(&field.selection) + 1;
    if n > limits.max_complexity {
        Err(format!(
            "query complexity {n} exceeds limit {}",
            limits.max_complexity
        ))
    } else {
        Ok(())
    }
}

fn complexity(sel: &Option<SelectionSet>) -> usize {
    match sel {
        None => 0,
        Some(set) => set.values().map(|n| 1 + complexity(&n.selection)).sum(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse_query;
    use crate::{GraphQLSchema, execute};
    use serde_json::Value;

    fn parse(q: &str) -> ParsedField {
        parse_query(q, &Value::Null).unwrap()
    }

    fn limits(max: usize) -> QueryLimits {
        QueryLimits {
            max_complexity: max,
        }
    }

    #[test]
    fn over_budget_query_is_rejected() {
        // 根 a + b + c + x + d + e = 6 个节点，预算 5
        let f = parse("{ a { b { c { x } } d { e } } }");
        let err = validate(&f, &limits(5)).unwrap_err();
        assert!(err.contains("complexity 6 exceeds limit 5"), "got: {err}");
    }

    #[test]
    fn under_budget_query_passes() {
        let f = parse("{ a { b { c } d { e } } }");
        assert!(validate(&f, &limits(5)).is_ok());
        assert!(validate(&f, &QueryLimits::default()).is_ok());
    }

    #[test]
    fn bare_field_costs_one() {
        let f = parse("{ ping }");
        assert!(validate(&f, &limits(1)).is_ok());
        assert!(validate(&f, &limits(0)).is_err());
    }

    #[test]
    fn execute_rejects_over_budget_before_resolving() {
        let schema = GraphQLSchema::new()
            .with_limits(limits(1))
            .query_fn("boom", |_v| {
                Box::pin(async { Ok(serde_json::json!("never")) })
            });
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt
            .block_on(execute(&schema, "{ boom { x } }", &Value::Null))
            .unwrap_err();
        assert!(
            err.iter().any(|e| e.contains("exceeds limit")),
            "got: {err:?}"
        );
    }

    #[test]
    fn deeply_nested_query_is_rejected_at_parse() {
        let deep = "{ a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a { a } } } } } } } } } } } } } } } } } } } } } } } } } } } } } } } } } } }";
        let err = parse_query(deep, &Value::Null).unwrap_err();
        assert!(err.contains("selection too deep"), "got: {err}");
    }

    #[test]
    fn alias_flood_is_rejected_at_parse() {
        // 解析器不支持别名：别名在预算校验前即被拒绝，无放大路径
        let err = parse_query("{ a: b c: d e: f g: h i: j k: l m: n }", &Value::Null).unwrap_err();
        assert!(err.contains("aliases not supported"), "got: {err}");
    }
}
