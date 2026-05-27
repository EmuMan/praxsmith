use std::fmt;

use anyhow::{Context, Result, bail};

use crate::{
    expressions::Expression,
    queries::{ActorRef, Query, RelationQuery},
    types::{FieldType, RelationTypeData},
    values::{Sentence, Value},
    world::{World, bindings::Bindings, goals::GoalMeasurement, simulation::Effect},
};

#[derive(Debug, Clone)]
pub struct Guarantees {
    items: Vec<RelationQuery>,
}

impl Guarantees {
    pub fn new(items: Vec<RelationQuery>) -> Self {
        Guarantees { items }
    }

    pub fn is_guaranteed(&self, relation: &RelationQuery) -> bool {
        self.items.iter().any(|guarantee| guarantee == relation)
    }

    pub fn push(&mut self, guarantee: RelationQuery) {
        self.items.push(guarantee);
    }

    pub fn push_many(&mut self, guarantees: Vec<RelationQuery>) -> usize {
        let count = guarantees.len();
        self.items.extend(guarantees);
        count
    }

    pub fn pop(&mut self) {
        self.items.pop();
    }

    pub fn pop_many(&mut self, count: usize) {
        for _ in 0..count {
            self.items.pop();
        }
    }

    pub fn merged_with(mut self, other: &Guarantees) -> Self {
        self.items.extend(other.items.clone());
        self
    }

    pub fn extend(&mut self, other: &Guarantees) {
        self.items.extend(other.items.clone());
    }
}

impl Default for Guarantees {
    fn default() -> Self {
        Guarantees { items: vec![] }
    }
}

struct ValidActors {
    items: Vec<String>,
}

impl ValidActors {
    pub fn new(items: Vec<String>) -> Self {
        ValidActors { items }
    }

    pub fn is_valid(&self, actor_id: &str) -> bool {
        self.items.iter().any(|valid_actor| valid_actor == actor_id)
    }

    pub fn validate_query(&self, query: &RelationQuery) -> Result<()> {
        for actor in query.get_all_actors() {
            if !self.is_valid(actor.symbol()) {
                bail!(
                    "actor {} is not in scope for query {}",
                    actor.symbol(),
                    query
                );
            }
        }
        Ok(())
    }

    pub fn push(&mut self, actor_id: String) -> Result<()> {
        if self.is_valid(&actor_id) {
            bail!("actor {} is bound more than once", actor_id);
        }
        self.items.push(actor_id);
        Ok(())
    }

    pub fn pop(&mut self) {
        self.items.pop();
    }
}

impl Default for ValidActors {
    fn default() -> Self {
        ValidActors { items: vec![] }
    }
}

#[derive(Debug, Clone)]
pub enum ResultType {
    Boolean {
        true_guarantees: Guarantees,
        false_guarantees: Guarantees,
    },
    Number,
    String,
    Variant,
    ActorRef,
}

impl ResultType {
    pub fn empty_boolean() -> Self {
        ResultType::Boolean {
            true_guarantees: Guarantees::default(),
            false_guarantees: Guarantees::default(),
        }
    }
}

impl fmt::Display for ResultType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ResultType::Boolean { .. } => write!(f, "Boolean"),
            ResultType::Number => write!(f, "Number"),
            ResultType::String => write!(f, "String"),
            ResultType::Variant => write!(f, "Variant"),
            ResultType::ActorRef => write!(f, "ActorRef"),
        }
    }
}

/// Runs a type check on the given expression, returning the resulting type.
///
/// `valid_actors` is the list of currently valid actor bindings, used to check
/// ActorRef values and queries. It should include all actors and any extra
/// bindings, introduced by practices, if there are any.
/// `self_id` is the sentence representing the "self" practice for the current
/// context, or None if there is no self.
/// `initial_guarantees` is any guarantees that should be applied to the
/// expression being checked, such as conditions that the world has already
/// verified in practices.
pub fn type_check(
    expression: &Expression,
    world: &World,
    valid_actors: &[String],
    self_id: Option<&Sentence>,
    initial_guarantees: Option<&mut Guarantees>,
) -> Result<ResultType> {
    let guarantees = match initial_guarantees {
        Some(guarantees) => guarantees,
        None => &mut Guarantees::default(),
    };
    let bindings = match self_id {
        Some(self_id) => Bindings::self_only(self_id.clone()),
        None => Bindings::default(),
    };
    type_check_helper(
        expression,
        world,
        guarantees,
        &mut ValidActors::new(valid_actors.to_vec()),
        &bindings,
    )
}

fn type_check_helper(
    expression: &Expression,
    world: &World,
    guarantees: &mut Guarantees,
    valid_actors: &mut ValidActors,
    bindings: &Bindings,
) -> Result<ResultType> {
    log::info!("type checking expression {}", expression);
    match expression {
        Expression::Value(value) => match value {
            Value::Number(_) => Ok(ResultType::Number),
            Value::Boolean(_) => Ok(ResultType::empty_boolean()),
            Value::Variant(_) => Ok(ResultType::Variant),
            Value::String(_) => Ok(ResultType::String),
            Value::ActorRef(r) => {
                if valid_actors.is_valid(r) {
                    Ok(ResultType::ActorRef)
                } else {
                    bail!("actor {} is not in scope", r)
                }
            }
            Value::Variable(sentence) => type_check_query(
                Query::parse(world, sentence, bindings)?,
                world,
                guarantees,
                valid_actors,
            ),
        },
        Expression::And(x, y) => {
            let x = type_check_helper(x, world, guarantees, valid_actors, bindings)?;
            let ResultType::Boolean {
                true_guarantees: x_true_guarantees,
                ..
            } = x
            else {
                bail!("And conditions must be boolean, got {}", x);
            };

            // We can evaluate the second condition under the assumption that
            // the first condition is true because of short-circuiting! It will
            // not be run otherwise!
            let added_count = guarantees.push_many(x_true_guarantees.items.clone());
            let y = type_check_helper(y, world, guarantees, valid_actors, bindings)?;
            guarantees.pop_many(added_count);
            match y {
                // True guarantees can be combined because if the whole
                // condition is true, both branches must be true. False
                // guarantees do not have this property because either branch
                // could be false.
                ResultType::Boolean {
                    true_guarantees: y_true_guarantees,
                    ..
                } => Ok(ResultType::Boolean {
                    true_guarantees: x_true_guarantees.merged_with(&y_true_guarantees),
                    false_guarantees: Guarantees::default(),
                }),
                other => bail!("And conditions must be boolean, got {}", other),
            }
        }
        Expression::Or(x, y) => {
            let x = type_check_helper(x, world, guarantees, valid_actors, bindings)?;
            let ResultType::Boolean {
                false_guarantees: x_false_guarantees,
                ..
            } = x
            else {
                bail!("Or conditions must be boolean, got {}", x);
            };

            // We can evaluate the second condition under the assumption that
            // the first condition is false because of short-circuiting! It
            // will not be run otherwise!
            let added_count = guarantees.push_many(x_false_guarantees.items.clone());
            let y = type_check_helper(y, world, guarantees, valid_actors, bindings)?;
            guarantees.pop_many(added_count);
            match y {
                // False guarantees can be combined because if the whole
                // condition is false, both branches must be false. True
                // guarantees do not have this property because either branch
                // could be true.
                ResultType::Boolean {
                    false_guarantees: y_false_guarantees,
                    ..
                } => Ok(ResultType::Boolean {
                    true_guarantees: Guarantees::default(),
                    false_guarantees: x_false_guarantees.merged_with(&y_false_guarantees),
                }),
                other => bail!("Or conditions must be boolean, got {}", other),
            }
        }
        Expression::Is(x, y) => {
            let x = type_check_helper(x, world, guarantees, valid_actors, bindings)?;
            let y = type_check_helper(y, world, guarantees, valid_actors, bindings)?;
            // The only condition is that they are the same type
            if std::mem::discriminant(&x) != std::mem::discriminant(&y) {
                bail!(
                    "Is conditions must compare values of the same type, got {} and {}",
                    x,
                    y
                );
            }
            // Can theoretically introduce dependent guarantees, but that's too
            // complicated for my tired brain right now.
            Ok(ResultType::empty_boolean())
        }
        Expression::LessThan(x, y)
        | Expression::GreaterThan(x, y)
        | Expression::LessThanOrEqual(x, y)
        | Expression::GreaterThanOrEqual(x, y) => {
            let x = type_check_helper(x, world, guarantees, valid_actors, bindings)?;
            let y = type_check_helper(y, world, guarantees, valid_actors, bindings)?;
            // Both sides must be numbers
            if !matches!(x, ResultType::Number) {
                bail!("comparison conditions must be numeric, got {}", x);
            }
            if !matches!(y, ResultType::Number) {
                bail!("comparison conditions must be numeric, got {}", y);
            }
            Ok(ResultType::empty_boolean())
        }
        Expression::Multiply(x, y) => {
            let x = type_check_helper(x, world, guarantees, valid_actors, bindings)?;
            let y = type_check_helper(y, world, guarantees, valid_actors, bindings)?;

            // Can be: number/number, number/string, number/boolean (unordered)
            match (x, y) {
                (ResultType::Number, ResultType::Number) => Ok(ResultType::Number),
                (ResultType::Number, ResultType::String)
                | (ResultType::String, ResultType::Number) => Ok(ResultType::String),
                (ResultType::Number, ResultType::Boolean { .. })
                | (ResultType::Boolean { .. }, ResultType::Number) => {
                    // Multiplication with boolean returns n or 0
                    Ok(ResultType::Number)
                }
                (x, y) => bail!(
                    "Multiply conditions must evaluate to either Number and Number, Number and String, or Number and Boolean, got {} and {}",
                    x,
                    y
                ),
            }
        }
        Expression::Divide(x, y) => {
            let x = type_check_helper(x, world, guarantees, valid_actors, bindings)?;
            let y = type_check_helper(y, world, guarantees, valid_actors, bindings)?;

            match (x, y) {
                (ResultType::Number, ResultType::Number) => Ok(ResultType::Number),
                (other_x, other_y) => bail!(
                    "Divide conditions must evaluate to numbers, got {} and {}",
                    other_x,
                    other_y
                ),
            }
        }
        Expression::Add(x, y) => {
            let x = type_check_helper(x, world, guarantees, valid_actors, bindings)?;
            let y = type_check_helper(y, world, guarantees, valid_actors, bindings)?;

            match (x, y) {
                (ResultType::Number, ResultType::Number) => Ok(ResultType::Number),
                (ResultType::String, ResultType::String) => Ok(ResultType::String),
                (ResultType::Number, ResultType::String)
                | (ResultType::String, ResultType::Number) => Ok(ResultType::String),
                (ResultType::Boolean { .. }, ResultType::String)
                | (ResultType::String, ResultType::Boolean { .. }) => Ok(ResultType::String),
                (ResultType::Variant, ResultType::String)
                | (ResultType::String, ResultType::Variant) => Ok(ResultType::String),
                (ResultType::ActorRef, ResultType::String)
                | (ResultType::String, ResultType::ActorRef) => Ok(ResultType::String),
                (other_x, other_y) => bail!(
                    "Add conditions must evaluate to either Number and Number or String and Any; got {} and {}",
                    other_x,
                    other_y
                ),
            }
        }
        Expression::Subtract(x, y) => {
            let x = type_check_helper(x, world, guarantees, valid_actors, bindings)?;
            let y = type_check_helper(y, world, guarantees, valid_actors, bindings)?;

            match (x, y) {
                (ResultType::Number, ResultType::Number) => Ok(ResultType::Number),
                (other_x, other_y) => bail!(
                    "Subtract conditions must evaluate to numbers, got {} and {}",
                    other_x,
                    other_y
                ),
            }
        }
        Expression::Not(expr) => {
            let res = type_check_helper(expr, world, guarantees, valid_actors, bindings)?;
            match res {
                // Just swap the true and false guarantees because the result
                // value is negated.
                ResultType::Boolean {
                    true_guarantees,
                    false_guarantees,
                } => Ok(ResultType::Boolean {
                    true_guarantees: false_guarantees,
                    false_guarantees: true_guarantees,
                }),
                other => bail!("Not conditions must be boolean, got {}", other),
            }
        }
        Expression::ForAll(new_symbol, expression) => {
            // The new symbol represents a new valid actor binding
            valid_actors.push(new_symbol.clone())?;
            let res = type_check_helper(expression, world, guarantees, valid_actors, bindings)?;
            valid_actors.pop();
            match res {
                ResultType::Boolean { .. } => Ok(ResultType::empty_boolean()),
                other => bail!("ForAll conditions must be boolean, got {}", other),
            }
        }
        Expression::Any(new_symbol, expression) => {
            // The new symbol represents a new valid actor binding
            valid_actors.push(new_symbol.clone())?;
            let res = type_check_helper(expression, world, guarantees, valid_actors, bindings)?;
            valid_actors.pop();
            match res {
                ResultType::Boolean { .. } => Ok(ResultType::empty_boolean()),
                other => bail!("Any conditions must be boolean, got {}", other),
            }
        }
        Expression::Count(new_symbol, expression) => {
            // The new symbol represents a new valid actor binding
            valid_actors.push(new_symbol.clone())?;
            let res = type_check_helper(expression, world, guarantees, valid_actors, bindings)?;
            valid_actors.pop();
            match res {
                ResultType::Boolean { .. } => Ok(ResultType::Number),
                other => bail!("Any conditions must be boolean, got {}", other),
            }
        }
        Expression::Aggregate {
            body, var, filter, ..
        } => {
            // The new symbol represents a new valid actor binding
            valid_actors.push(var.clone())?;
            let filter_res = type_check_helper(filter, world, guarantees, valid_actors, bindings)?;
            valid_actors.pop();

            let true_guarantees = match filter_res {
                ResultType::Boolean {
                    true_guarantees, ..
                } => true_guarantees,
                other => bail!("Aggregate filter must be boolean, got {}", other),
            };

            // We can evaluate the aggregate body under the assumptions of the
            // filter because any value being evaluated has passed the filter.
            // Push valid actor again here to keep these atomic.
            valid_actors.push(var.clone())?;
            let added_count = guarantees.push_many(true_guarantees.items.clone());
            let body_res = type_check_helper(body, world, guarantees, valid_actors, bindings)?;
            valid_actors.pop();
            guarantees.pop_many(added_count);

            match body_res {
                ResultType::Number => Ok(ResultType::Number),
                other => bail!("Aggregate bodies must be numeric, got {}", other),
            }
        }
    }
}

/// Runs a type check on the given query, returning the resulting type. Used as
/// a helper for type checking expressions that contain queries.
///
/// Fielded queries consume guarantees because they require the relation to
/// exist, while unfielded queries introduce a true-guarantee for themselves
/// because they act as an existence check.
fn type_check_query(
    query: Query,
    world: &World,
    guarantees: &Guarantees,
    valid_actors: &ValidActors,
) -> Result<ResultType> {
    match query {
        Query::Fielded(relation_query, field_name) => {
            valid_actors.validate_query(&relation_query)?;

            // Fielded relationships require a guarantee because the lookup is
            // not possible with a nonexistent relationship.
            if !guarantees.is_guaranteed(&relation_query) {
                bail!(
                    "cannot access field {} of {} without guarantee that it exists",
                    field_name,
                    relation_query
                );
            }

            let relation_type = world
                .get_relation_type_map()
                .get_type(relation_query.type_name())
                .unwrap();

            let field = relation_type.fields.get(&field_name).with_context(|| {
                format!(
                    "field {} does not exist on type {}",
                    field_name, relation_type.name
                )
            })?;

            match field {
                FieldType::NumberRange(..) => Ok(ResultType::Number),
                FieldType::VariantList(..) => Ok(ResultType::Variant),
                FieldType::ActorRef => Ok(ResultType::ActorRef),
                FieldType::String => Ok(ResultType::String),
                FieldType::Boolean => Ok(ResultType::empty_boolean()),
            }
        }
        Query::Unfielded(relation_query) => {
            valid_actors.validate_query(&relation_query)?;

            // Unfielded queries always return booleans, and we can introduce
            // a true-guarantee for it because it acts as an existence check.
            Ok(ResultType::Boolean {
                true_guarantees: Guarantees::new(vec![relation_query]),
                false_guarantees: Guarantees::default(),
            })
        }
    }
}

/// Runs a type check on the world to ensure that all conditions/effects in
/// practices and actor goals are well-formed.
pub fn type_check_world(world: &World) -> Result<()> {
    let global_valid_actors = get_valid_actors(world, &[]);

    for relation_type in world.get_relation_type_map().iter_types() {
        match &relation_type.data {
            RelationTypeData::Practice {
                self_id,
                actions,
                params,
            } => {
                let practice_valid_actors = get_valid_actors(world, &params);

                // Because we are in a practice, we can guarantee that whatever
                // `self` resolves to will always exist.
                let mut guarantees = Guarantees::new(vec![RelationQuery::Practice {
                    type_name: relation_type.name.clone(),
                    participants: params
                        .iter()
                        .map(|param| ActorRef::Free(param.clone()))
                        .collect(),
                }]);

                for action in actions.iter() {
                    // any guarantees from conditions can be applied to
                    // outcomes, since they can only run if all conditions are
                    // true. they will be added to `guarantees` in the process
                    // of type checking.
                    type_check_conditions(
                        &action.conditions,
                        world,
                        params,
                        Some(self_id),
                        &mut guarantees,
                    )
                    .with_context(|| {
                        format!(
                            "type checking conditions of action {} in practice {}",
                            action.name, relation_type.name
                        )
                    })?;

                    for effect in action.effects.iter() {
                        type_check_effect(
                            effect,
                            world,
                            params,
                            Some(self_id),
                            &practice_valid_actors,
                            &mut guarantees,
                        )
                        .with_context(|| {
                            format!(
                                "type checking outcome '{}' of action {} in practice {}",
                                effect, action.name, relation_type.name
                            )
                        })?;
                    }
                }
            }
            _ => {}
        }
    }

    for (actor_id, actor) in world.iter_actors() {
        for goal in actor.goals.iter() {
            match goal.measurement {
                GoalMeasurement::Exists => expect_type(
                    &goal.expression,
                    world,
                    &global_valid_actors,
                    None,
                    ResultType::empty_boolean(),
                ),
                GoalMeasurement::Delta => expect_type(
                    &goal.expression,
                    world,
                    &global_valid_actors,
                    None,
                    ResultType::Number,
                ),
            }
            .with_context(|| {
                format!(
                    "type checking expression of goal {} for actor {}",
                    goal, actor_id
                )
            })?;
        }
    }

    Ok(())
}

fn get_valid_actors(world: &World, extra_bindings: &[String]) -> Vec<String> {
    world
        .iter_actors()
        .map(|(actor_id, _)| actor_id.clone())
        .chain(extra_bindings.iter().cloned())
        .collect()
}

fn expect_type(
    expression: &Expression,
    world: &World,
    valid_actors: &[String],
    self_id: Option<&Sentence>,
    expected: ResultType,
) -> Result<()> {
    let actual = type_check(expression, world, valid_actors, self_id, None).with_context(|| {
        format!(
            "type checking expression {} expected to be {}",
            expression, expected
        )
    })?;

    if std::mem::discriminant(&actual) != std::mem::discriminant(&expected) {
        bail!(
            "expected expression {} to be of type {}, but got {}",
            expression,
            expected,
            actual
        );
    }

    Ok(())
}

/// Runs a type check on the given list of condition expressions, building on
/// `guarantees` in the process with any guarantees introduced by the conditions.
fn type_check_conditions(
    expressions: &[Expression],
    world: &World,
    extra_bindings: &[String],
    self_id: Option<&Sentence>,
    guarantees: &mut Guarantees,
) -> Result<()> {
    // Building on guarantees allows us to carry guarantees from one condition
    // to the next! On top of passing the self guarantee through.
    for expression in expressions.iter() {
        let eval = type_check(expression, world, extra_bindings, self_id, Some(guarantees))
            .with_context(|| format!("type checking condition expression {}", expression))?;
        match eval {
            ResultType::Boolean {
                true_guarantees, ..
            } => {
                guarantees.extend(&true_guarantees);
            }
            other => bail!(
                "expected condition expression {} to be boolean, but got {}",
                expression,
                other
            ),
        }
    }

    Ok(())
}

fn type_check_effect(
    effect: &Effect,
    world: &World,
    extra_bindings: &[String],
    self_id: Option<&Sentence>,
    valid_actors: &[String],
    guarantees: &mut Guarantees,
) -> Result<()> {
    match effect {
        Effect::Broadcast(expr) | Effect::Say(expr) => {
            type_check(expr, world, extra_bindings, self_id, Some(guarantees))
                .with_context(|| format!("type checking broadcast expression: {}", expr))?;
            Ok(())
        }
        Effect::Activate(expr) | Effect::Deactivate(expr) => {
            let res = type_check(expr, world, extra_bindings, self_id, Some(guarantees))
                .with_context(|| format!("type checking activation expression: {}", expr))?;
            match res {
                ResultType::ActorRef => Ok(()),
                other => bail!(
                    "expected activation expression {} to be ActorRef, but got {}",
                    expr,
                    other
                ),
            }
        }
        Effect::Update(sentence, expr) => {
            let bindings = match self_id {
                Some(self_id) => Bindings::self_only(self_id.clone()),
                None => Bindings::default(),
            };
            let field_type = type_check_query(
                Query::parse(world, sentence, &bindings)?,
                world,
                guarantees,
                &ValidActors::new(valid_actors.to_vec()),
            )
            .with_context(|| format!("type checking update query: {}", sentence))?;

            let expr_type = type_check(expr, world, extra_bindings, self_id, Some(guarantees))
                .with_context(|| format!("type checking update expression: {}", expr))?;

            if std::mem::discriminant(&field_type) != std::mem::discriminant(&expr_type) {
                bail!(
                    "expected update expression {} to be of type {}, but got {}",
                    expr,
                    field_type,
                    expr_type
                );
            }

            Ok(())
        }
        Effect::Increase(sentence, ..) | Effect::Cycle(sentence, ..) => {
            let bindings = match self_id {
                Some(self_id) => Bindings::self_only(self_id.clone()),
                None => Bindings::default(),
            };
            let result_type = type_check_query(
                Query::parse(world, sentence, &bindings)?,
                world,
                guarantees,
                &ValidActors::new(valid_actors.to_vec()),
            )
            .with_context(|| format!("type checking increment query: {}", sentence))?;

            match result_type {
                ResultType::Number | ResultType::Variant => Ok(()),
                other => bail!(
                    "expected increment query {} to be Number or Variant, but got {}",
                    sentence,
                    other
                ),
            }
        }
        _ => Ok(()),
    }
}
