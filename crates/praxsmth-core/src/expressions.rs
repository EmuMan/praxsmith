use std::fmt;

use anyhow::{Context, Result, bail};

use crate::{
    queries::Query,
    values::{Constant, Value},
    world::{World, bindings::Bindings},
};

#[derive(Debug, Clone)]
pub enum Expression {
    Value(Value),
    /// Boolean, Boolean -> Boolean
    And(Box<Expression>, Box<Expression>),
    /// Boolean, Boolean -> Boolean
    Or(Box<Expression>, Box<Expression>),
    /// T, T -> Boolean
    Is(Box<Expression>, Box<Expression>),
    /// Boolean -> Boolean
    Not(Box<Expression>),
    /// Number, Number -> Boolean
    LessThan(Box<Expression>, Box<Expression>),
    /// Number, Number -> Boolean
    GreaterThan(Box<Expression>, Box<Expression>),
    /// Number, Number -> Boolean
    LessThanOrEqual(Box<Expression>, Box<Expression>),
    /// Number, Number -> Boolean
    GreaterThanOrEqual(Box<Expression>, Box<Expression>),
    /// Number, Number -> Number
    Multiply(Box<Expression>, Box<Expression>),
    /// Number, Number -> Number
    Divide(Box<Expression>, Box<Expression>),
    /// Number, Number -> Number
    Add(Box<Expression>, Box<Expression>),
    /// Number, Number -> Number
    Subtract(Box<Expression>, Box<Expression>),
    /// Boolean... -> Boolean (`for all X, Y` = Y must hold for every binding of X)
    ForAll(String, Box<Expression>),
    /// Boolean... -> Boolean (`any X where Y` = there exists some binding of X for which Y holds)
    Any(String, Box<Expression>),
    /// Number (`count SYM where FILTER` = how many bindings of SYM satisfy FILTER)
    Count(String, Box<Expression>),
    /// Number (`OP BODY across SYM where FILTER` = reduce BODY over the bindings
    /// of SYM that satisfy FILTER). With no matching bindings, evaluates to 0.
    Aggregate {
        op: AggregateOp,
        /// Numeric expression evaluated once per matching binding of `var`.
        body: Box<Expression>,
        /// The bound variable iterated over.
        var: String,
        /// Boolean expression selecting which bindings of `var` contribute.
        filter: Box<Expression>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AggregateOp {
    Sum,
    Average,
    Min,
    Max,
}

fn evaluate_inequality<F>(
    world: &World,
    bindings: &Bindings,
    x: &Expression,
    y: &Expression,
    cmp_fn: F,
) -> Result<Constant>
where
    F: Fn(f64, f64) -> bool,
{
    let x = x.evaluate(world, bindings)?;
    let y = y.evaluate(world, bindings)?;

    let Constant::Number(x) = x else {
        bail!(
            "Left-hand side of inequality must evaluate to number, got {:?}",
            x
        );
    };
    let Constant::Number(y) = y else {
        bail!(
            "Right-hand side of inequality must evaluate to number, got {:?}",
            y
        );
    };

    Ok(Constant::Boolean(cmp_fn(x, y)))
}

fn evaluate_binary_numeric_op<F>(
    world: &World,
    bindings: &Bindings,
    x: &Expression,
    y: &Expression,
    op_fn: F,
) -> Result<Constant>
where
    F: Fn(f64, f64) -> f64,
{
    let x = x.evaluate(world, bindings)?;
    let y = y.evaluate(world, bindings)?;

    let Constant::Number(x) = x else {
        bail!(
            "Left-hand side of numeric operation must evaluate to number, got {:?}",
            x
        );
    };
    let Constant::Number(y) = y else {
        bail!(
            "Right-hand side of numeric operation must evaluate to number, got {:?}",
            y
        );
    };

    Ok(Constant::Number(op_fn(x, y)))
}

impl Expression {
    /// Evaluates an expression to a single constant value.
    ///
    /// Returns an error if there are any free variable assignments within the
    /// expression tree. Solve for these with `World::solve_for_free_vars(...)`
    /// first before passing the bindings into this function if you need to
    /// avoid this problem.
    pub fn evaluate(&self, world: &World, bindings: &Bindings) -> Result<Constant> {
        match self {
            Expression::Value(value) => match value {
                Value::Number(n) => Ok(Constant::Number(*n)),
                Value::Boolean(b) => Ok(Constant::Boolean(*b)),
                Value::Variant(v) => Ok(Constant::Variant(v.clone())),
                Value::String(s) => Ok(Constant::String(s.clone())),
                Value::ActorRef(r) => Ok(Constant::ActorRef(bindings.get_or_same(r))),
                Value::Variable(sentence) => Query::parse(world, sentence, bindings)?
                    .apply_bindings(bindings)
                    .evaluate_in_world(world)
                    .with_context(|| {
                        format!(
                            "evaluating variable for expression with sentence {:?}",
                            sentence
                        )
                    }),
            },

            Expression::And(x, y) => {
                let x = x.evaluate(world, bindings)?;
                let Constant::Boolean(x) = x else {
                    bail!("And conditions must evaluate to boolean, got {:?}", x);
                };
                if !x {
                    // Short circuit, important behavior!
                    return Ok(Constant::Boolean(false));
                }

                let y = y.evaluate(world, bindings)?;
                match y {
                    Constant::Boolean(y) => Ok(Constant::Boolean(y)),
                    other => bail!("And conditions must evaluate to boolean, got {:?}", other),
                }
            }

            Expression::Or(x, y) => {
                let x = x.evaluate(world, bindings)?;
                let Constant::Boolean(x) = x else {
                    bail!("Or conditions must evaluate to boolean, got {:?}", x);
                };
                if x {
                    // Short circuit, important behavior!
                    return Ok(Constant::Boolean(true));
                }

                let y = y.evaluate(world, bindings)?;
                match y {
                    Constant::Boolean(y) => Ok(Constant::Boolean(y)),
                    other => bail!("Or conditions must evaluate to boolean, got {:?}", other),
                }
            }

            Expression::Is(x, y) => {
                let x = x.evaluate(world, bindings)?;
                let y = y.evaluate(world, bindings)?;
                Ok(Constant::Boolean(x == y))
            }

            Expression::LessThan(x, y) => evaluate_inequality(world, bindings, x, y, |a, b| a < b),
            Expression::GreaterThan(x, y) => {
                evaluate_inequality(world, bindings, x, y, |a, b| a > b)
            }
            Expression::LessThanOrEqual(x, y) => {
                evaluate_inequality(world, bindings, x, y, |a, b| a <= b)
            }
            Expression::GreaterThanOrEqual(x, y) => {
                evaluate_inequality(world, bindings, x, y, |a, b| a >= b)
            }

            Expression::Multiply(x, y) => {
                evaluate_binary_numeric_op(world, bindings, x, y, |a, b| a * b)
            }
            Expression::Divide(x, y) => {
                evaluate_binary_numeric_op(world, bindings, x, y, |a, b| a / b)
            }
            Expression::Add(x, y) => {
                evaluate_binary_numeric_op(world, bindings, x, y, |a, b| a + b)
            }
            Expression::Subtract(x, y) => {
                evaluate_binary_numeric_op(world, bindings, x, y, |a, b| a - b)
            }

            Expression::Not(x) => {
                let res = x.evaluate(world, bindings)?;
                match res {
                    Constant::Boolean(b) => Ok(Constant::Boolean(!b)),
                    other => bail!("Not condition must evaluate to boolean, got {:?}", other),
                }
            }

            Expression::ForAll(new_symbol, inner) => {
                for (actor_id, _) in world.iter_actors() {
                    let new_bindings =
                        bindings.with([(new_symbol.clone(), actor_id.clone())].into());
                    match inner.evaluate(world, &new_bindings)? {
                        Constant::Boolean(true) => continue,
                        Constant::Boolean(false) => {
                            return Ok(Constant::Boolean(false));
                        }
                        other => {
                            bail!("ForAll condition must evaluate to boolean, got {:?}", other)
                        }
                    }
                }
                Ok(Constant::Boolean(true))
            }

            Expression::Any(new_symbol, inner) => {
                for (actor_id, _) in world.iter_actors() {
                    let new_bindings =
                        bindings.with([(new_symbol.clone(), actor_id.clone())].into());
                    match inner.evaluate(world, &new_bindings)? {
                        Constant::Boolean(true) => {
                            return Ok(Constant::Boolean(true));
                        }
                        Constant::Boolean(false) => continue,
                        other => {
                            bail!("Any condition must evaluate to boolean, got {:?}", other)
                        }
                    }
                }
                Ok(Constant::Boolean(false))
            }

            Expression::Count(new_symbol, inner) => {
                let mut count = 0;
                for (actor_id, _) in world.iter_actors() {
                    let new_bindings =
                        bindings.with([(new_symbol.clone(), actor_id.clone())].into());
                    match inner.evaluate(world, &new_bindings)? {
                        Constant::Boolean(true) => count += 1,
                        Constant::Boolean(false) => continue,
                        other => {
                            bail!("Count condition must evaluate to boolean, got {:?}", other)
                        }
                    }
                }
                Ok(Constant::Number(count.into()))
            }

            Expression::Aggregate {
                op,
                body,
                var,
                filter,
            } => {
                let mut values = vec![];

                for (actor_id, _) in world.iter_actors() {
                    let new_bindings = bindings.with([(var.clone(), actor_id.clone())].into());
                    match filter.evaluate(world, &new_bindings)? {
                        Constant::Boolean(true) => {
                            let value = body.evaluate(world, &new_bindings)?;
                            match value {
                                Constant::Number(n) => values.push(n),
                                other => {
                                    bail!("Aggregate body must evaluate to number, got {:?}", other)
                                }
                            }
                        }
                        Constant::Boolean(false) => continue,
                        other => {
                            bail!("Aggregate filter must evaluate to boolean, got {:?}", other)
                        }
                    }
                }

                Ok(match op {
                    AggregateOp::Sum => Constant::Number(values.into_iter().sum()),
                    AggregateOp::Average => {
                        let count = values.len();
                        if count == 0 {
                            Constant::Number(0.0)
                        } else {
                            Constant::Number(values.into_iter().sum::<f64>() / count as f64)
                        }
                    }
                    AggregateOp::Min => values
                        .into_iter()
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .map(Constant::Number)
                        .unwrap_or(Constant::Number(0.0)),
                    AggregateOp::Max => values
                        .into_iter()
                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                        .map(Constant::Number)
                        .unwrap_or(Constant::Number(0.0)),
                })
            }
        }
    }
}

impl fmt::Display for Expression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expression::Value(value) => write!(f, "{}", value),
            Expression::And(x, y) => write!(f, "({} and {})", x, y),
            Expression::Or(x, y) => write!(f, "({} or {})", x, y),
            Expression::Is(x, y) => write!(f, "({} is {})", x, y),
            Expression::LessThan(x, y) => write!(f, "({} < {})", x, y),
            Expression::GreaterThan(x, y) => write!(f, "({} > {})", x, y),
            Expression::LessThanOrEqual(x, y) => write!(f, "({} <= {})", x, y),
            Expression::GreaterThanOrEqual(x, y) => write!(f, "({} >= {})", x, y),
            Expression::Multiply(x, y) => write!(f, "({} * {})", x, y),
            Expression::Divide(x, y) => write!(f, "({} / {})", x, y),
            Expression::Add(x, y) => write!(f, "({} + {})", x, y),
            Expression::Subtract(x, y) => write!(f, "({} - {})", x, y),
            Expression::Not(x) => write!(f, "(not {})", x),
            Expression::ForAll(var, inner) => write!(f, "(for all {}, {})", var, inner),
            Expression::Any(var, inner) => write!(f, "(any {} where {})", var, inner),
            Expression::Count(var, filter) => write!(f, "(count {} where {})", var, filter),
            Expression::Aggregate {
                op,
                body,
                var,
                filter,
            } => {
                let op_str = match op {
                    AggregateOp::Sum => "sum",
                    AggregateOp::Average => "average",
                    AggregateOp::Min => "min",
                    AggregateOp::Max => "max",
                };
                write!(f, "({} {} across {} where {})", op_str, body, var, filter)
            }
        }
    }
}
