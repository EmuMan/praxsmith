use std::fs;

use pest::{
    Parser,
    error::Error,
    iterators::Pair,
    pratt_parser::{Assoc, Op, PrattParser},
};
use pest_derive::Parser;

use crate::{
    expressions::{AggregateOp, Expression},
    types::{FieldType, RelationType},
    values::{Constant, Sentence, Value},
    world::simulation::Effect,
};

pub mod types;
pub mod world;

#[derive(Parser)]
#[grammar = "praxsmth.pest"]
struct PraxsmthParser;

fn parse_string(pair: Pair<Rule>) -> String {
    // pair is Rule::string
    pair.as_str().trim_matches('"').to_string()
}

fn parse_sentence(pair: Pair<Rule>) -> Sentence {
    // pair is Rule::sentence
    pair.into_inner()
        .map(|token| token.as_str().to_string())
        .collect::<Vec<String>>()
        .into()
}

fn parse_value(pair: Pair<Rule>) -> Value {
    match pair.as_rule() {
        Rule::number => Value::Number(pair.as_str().parse().unwrap()),
        Rule::string => Value::String(parse_string(pair)),
        Rule::sentence => {
            let parts = parse_sentence(pair);
            if parts.len() == 1 {
                Value::Variant(parts.into_iter().next().unwrap())
            } else {
                Value::Variable(parts)
            }
        }
        _ => unreachable!(),
    }
}

pub fn parse_constant(pair: Pair<Rule>) -> Constant {
    match pair.as_rule() {
        Rule::number => Constant::Number(pair.as_str().parse().unwrap()),
        Rule::string => Constant::String(parse_string(pair)),
        Rule::var_name => Constant::Variant(pair.as_str().to_string()),
        _ => unreachable!(),
    }
}

fn parse_field(pair: Pair<Rule>) -> FieldType {
    // pair is either Rule::number_range or Rule::variant_list
    match pair.as_rule() {
        Rule::number_range => {
            // number_range is: number ~ ".." ~ number
            let mut numbers = pair.into_inner();
            let start: f64 = numbers.next().unwrap().as_str().parse().unwrap();
            let end: f64 = numbers.next().unwrap().as_str().parse().unwrap();
            FieldType::NumberRange(start, end)
        }
        Rule::variant_list => {
            // variant_list is: var_name ~ ("|" ~ var_name)+
            let variants = pair
                .into_inner()
                .map(|var| var.as_str().to_string())
                .collect();
            FieldType::VariantList(variants)
        }
        _ => unreachable!(),
    }
}

pub fn test_parse() {
    let unparsed_types = fs::read_to_string("types.txt").expect("cannot read file");

    let values: Vec<RelationType> =
        types::parse_types(&unparsed_types).expect("unsuccessful parse");

    println!(
        "Types Output:\n\n{}",
        values
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );

    let unparsed_world = fs::read_to_string("world.txt").expect("cannot read file");

    let world_values = world::parse_world(&unparsed_world).expect("unsuccessful parse");

    println!(
        "\nWorld Output:\n\n{}",
        world_values
            .iter()
            .map(|v| v.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    );
}

/// Parse a single effect (e.g. `set agent.likes { amount: 5 }`) from a string.
pub fn parse_effect_str(input_str: &str) -> Result<Effect, Error<Rule>> {
    let mut pairs = PraxsmthParser::parse(Rule::parse_effect, input_str)?;
    // parse_effect is silent and anchored: SOI ~ effect ~ EOI
    let effect_pair = pairs.next().unwrap();
    Ok(types::parse_effect(effect_pair))
}

/// Parse a single expression (e.g. `a is b and not c`) from a string.
pub fn parse_expression_str(input_str: &str) -> Result<Expression, Error<Rule>> {
    let mut pairs = PraxsmthParser::parse(Rule::parse_expression, input_str)?;
    let expression_pair = pairs.next().unwrap();
    let pratt = build_expression_pratt();
    Ok(parse_expression(expression_pair, &pratt))
}

pub fn build_expression_pratt() -> PrattParser<Rule> {
    PrattParser::new()
        // Quantifiers bind loosest, so the body extends as far right as
        // possible (e.g. `for all x, a and b` is `for all x, (a and b)`).
        // The first `.op` call has the lowest precedence.
        .op(Op::prefix(Rule::forall) | Op::prefix(Rule::any_where))
        .op(Op::infix(Rule::and, Assoc::Left) | Op::infix(Rule::or, Assoc::Left))
        .op(Op::infix(Rule::is, Assoc::Left))
        .op(Op::prefix(Rule::not))
}

/// Parses `count SYM where FILTER`. Inner pairs are the keyword tokens
/// (skipped), the bound `var_name`, and the FILTER `expression`.
fn parse_agg_count(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> Expression {
    let mut var = String::new();
    let mut filter = None;
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::var_name => var = inner.as_str().to_string(),
            Rule::expression => filter = Some(parse_expression(inner, pratt)),
            _ => {} // keyword tokens
        }
    }
    Expression::Count(
        var,
        Box::new(filter.expect("agg_count is missing its filter")),
    )
}

/// Parses `(sum | average | min | max) BODY across SYM where FILTER`. The two
/// `expression` children appear in source order: BODY first, then FILTER.
fn parse_agg_reduce(pair: Pair<Rule>, pratt: &PrattParser<Rule>) -> Expression {
    let mut op = None;
    let mut var = String::new();
    let mut exprs = Vec::with_capacity(2);
    for inner in pair.into_inner() {
        match inner.as_rule() {
            Rule::agg_op => {
                op = Some(match inner.as_str() {
                    "sum" => AggregateOp::Sum,
                    "average" => AggregateOp::Average,
                    "min" => AggregateOp::Min,
                    "max" => AggregateOp::Max,
                    other => unreachable!("unexpected aggregate op {:?}", other),
                });
            }
            Rule::var_name => var = inner.as_str().to_string(),
            Rule::expression => exprs.push(parse_expression(inner, pratt)),
            _ => {} // keyword tokens
        }
    }
    let mut exprs = exprs.into_iter();
    let body = exprs.next().expect("agg_reduce is missing its body");
    let filter = exprs.next().expect("agg_reduce is missing its filter");
    Expression::Aggregate {
        op: op.expect("agg_reduce is missing its operator"),
        body: Box::new(body),
        var,
        filter: Box::new(filter),
    }
}

pub fn parse_expression(pairs: Pair<Rule>, pratt: &PrattParser<Rule>) -> Expression {
    pratt
        .map_primary(|primary| match primary.as_rule() {
            Rule::agg_count => parse_agg_count(primary, pratt),
            Rule::agg_reduce => parse_agg_reduce(primary, pratt),
            _ => Expression::Value(parse_value(primary)),
        })
        .map_prefix(|op, rhs| match op.as_rule() {
            Rule::not => Expression::Not(Box::new(rhs)),
            Rule::forall => {
                // `for all X, <body>` — first inner pair is the bound var_name.
                let var = op.into_inner().next().unwrap().as_str().to_string();
                Expression::ForAll(var, Box::new(rhs))
            }
            Rule::any_where => {
                // `any X where <body>` — first inner pair is the bound var_name.
                let var = op.into_inner().next().unwrap().as_str().to_string();
                Expression::Any(var, Box::new(rhs))
            }
            _ => unreachable!(),
        })
        .map_infix(|lhs, op, rhs| match op.as_rule() {
            Rule::and => Expression::And(Box::new(lhs), Box::new(rhs)),
            Rule::or => Expression::Or(Box::new(lhs), Box::new(rhs)),
            Rule::is => Expression::Is(Box::new(lhs), Box::new(rhs)),
            _ => unreachable!(),
        })
        .parse(pairs.into_inner())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_effect() {
        // `set` wraps a declaration, so this also exercises declaration parsing.
        let effect = parse_effect_str("set agent.likes { amount: 5 }").unwrap();
        match effect {
            Effect::Set(decl) => {
                assert_eq!(decl.sentence, Sentence::from_strs(&["agent", "likes"]));
                assert_eq!(decl.fields.get("amount"), Some(&Constant::Number(5.0)));
            }
            other => panic!("expected Effect::Set, got {:?}", other),
        }
    }

    #[test]
    fn parses_simple_effect() {
        assert!(matches!(
            parse_effect_str("activate guard").unwrap(),
            Effect::Activate(_)
        ));
    }

    #[test]
    fn rejects_effect_with_trailing_junk() {
        assert!(parse_effect_str("activate guard extra").is_err());
    }

    #[test]
    fn parses_expression() {
        // Just assert it parses; the AST shape is covered by the grammar tests.
        let expr = parse_expression_str("a is b and not c").unwrap();
        assert!(matches!(expr, Expression::And(_, _)));
    }

    #[test]
    fn rejects_expression_with_trailing_junk() {
        assert!(parse_expression_str("a is b )").is_err());
    }

    #[test]
    fn parses_forall_quantifier() {
        let expr = parse_expression_str("for all x, x is happy").unwrap();
        match expr {
            Expression::ForAll(var, body) => {
                assert_eq!(var, "x");
                assert!(matches!(*body, Expression::Is(_, _)));
            }
            other => panic!("expected ForAll, got {:?}", other),
        }
    }

    #[test]
    fn parses_any_where_quantifier() {
        let expr = parse_expression_str("any x where x is happy").unwrap();
        match expr {
            Expression::Any(var, body) => {
                assert_eq!(var, "x");
                assert!(matches!(*body, Expression::Is(_, _)));
            }
            other => panic!("expected Any, got {:?}", other),
        }
    }

    #[test]
    fn quantifier_body_is_greedy() {
        // Quantifiers bind loosest, so the body should swallow the trailing
        // `and`, yielding ForAll(x, And(..)) rather than And(ForAll(x, ..), ..).
        let expr = parse_expression_str("for all x, a and b").unwrap();
        match expr {
            Expression::ForAll(var, body) => {
                assert_eq!(var, "x");
                assert!(matches!(*body, Expression::And(_, _)));
            }
            other => panic!("expected ForAll wrapping And, got {:?}", other),
        }
    }

    #[test]
    fn parses_quantifier_under_not() {
        // `not` binds tighter than the quantifier, so this is ForAll(x, Not(..)).
        let expr = parse_expression_str("for all x, not x").unwrap();
        match expr {
            Expression::ForAll(var, body) => {
                assert_eq!(var, "x");
                assert!(matches!(*body, Expression::Not(_)));
            }
            other => panic!("expected ForAll wrapping Not, got {:?}", other),
        }
    }

    #[test]
    fn parses_nested_quantifiers() {
        let expr = parse_expression_str("for all x, any y where x is y").unwrap();
        match expr {
            Expression::ForAll(var, body) => {
                assert_eq!(var, "x");
                match *body {
                    Expression::Any(inner_var, inner_body) => {
                        assert_eq!(inner_var, "y");
                        assert!(matches!(*inner_body, Expression::Is(_, _)));
                    }
                    other => panic!("expected nested Any, got {:?}", other),
                }
            }
            other => panic!("expected outer ForAll, got {:?}", other),
        }
    }

    #[test]
    fn quantifier_keywords_do_not_swallow_identifiers() {
        // `anything` must not be lexed as the `any` quantifier; it is a plain
        // value, so the whole input has no operator and parses as a value.
        let expr = parse_expression_str("anything").unwrap();
        assert!(matches!(expr, Expression::Value(_)));
    }

    #[test]
    fn parses_count() {
        let expr = parse_expression_str("count x where x.likes.alice").unwrap();
        match expr {
            Expression::Count(var, filter) => {
                assert_eq!(var, "x");
                assert!(matches!(*filter, Expression::Value(_)));
            }
            other => panic!("expected Count, got {:?}", other),
        }
    }

    #[test]
    fn parses_sum_across() {
        let expr = parse_expression_str("sum x.likes.alice.strength across x where x.likes.alice")
            .unwrap();
        match expr {
            Expression::Aggregate {
                op,
                body,
                var,
                filter,
            } => {
                assert_eq!(op, AggregateOp::Sum);
                assert_eq!(var, "x");
                assert!(matches!(*body, Expression::Value(_)));
                assert!(matches!(*filter, Expression::Value(_)));
            }
            other => panic!("expected Aggregate, got {:?}", other),
        }
    }

    #[test]
    fn parses_all_reduce_ops() {
        for (kw, expected) in [
            ("sum", AggregateOp::Sum),
            ("average", AggregateOp::Average),
            ("min", AggregateOp::Min),
            ("max", AggregateOp::Max),
        ] {
            let src = format!("{} a.b across x where x.c", kw);
            match parse_expression_str(&src).unwrap() {
                Expression::Aggregate { op, .. } => assert_eq!(op, expected),
                other => panic!("expected Aggregate for {:?}, got {:?}", kw, other),
            }
        }
    }

    #[test]
    fn aggregate_composes_with_operators() {
        // Aggregates are numeric primaries, so they sit as an operand of `is`.
        // The `where` filter is greedy, so the comparison must come before it:
        // `count x where y` then `is 3` would bind `is 3` into the filter, so
        // here we put the aggregate on the right of `is`.
        let expr = parse_expression_str("3 is count x where x.c").unwrap();
        match expr {
            Expression::Is(lhs, rhs) => {
                assert!(matches!(*lhs, Expression::Value(_)));
                assert!(matches!(*rhs, Expression::Count(_, _)));
            }
            other => panic!("expected Is wrapping Count, got {:?}", other),
        }
    }

    #[test]
    fn aggregate_keywords_do_not_swallow_identifiers() {
        // `counter` / `summary` must not be lexed as `count` / `sum`.
        assert!(matches!(
            parse_expression_str("counter").unwrap(),
            Expression::Value(_)
        ));
        assert!(matches!(
            parse_expression_str("summary").unwrap(),
            Expression::Value(_)
        ));
    }
}
