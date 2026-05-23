use anyhow::{Context, Result, bail};

use crate::{
    expressions::Expression,
    queries::{Query, RelationQuery},
    types::FieldType,
    values::Value,
    world::{World, bindings::Bindings},
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
}

impl Default for Guarantees {
    fn default() -> Self {
        Guarantees { items: vec![] }
    }
}

struct ValidAgents {
    items: Vec<String>,
}

impl ValidAgents {
    pub fn new(items: Vec<String>) -> Self {
        ValidAgents { items }
    }

    pub fn is_valid(&self, agent_id: &str) -> bool {
        self.items.iter().any(|valid_agent| valid_agent == agent_id)
    }

    pub fn validate_query(&self, query: &RelationQuery) -> Result<()> {
        for agent in query.get_all_agents() {
            if !self.is_valid(agent.symbol()) {
                bail!(
                    "agent {} is not in scope for query {:?}",
                    agent.symbol(),
                    query
                );
            }
        }
        Ok(())
    }

    pub fn push(&mut self, agent_id: String) -> Result<()> {
        if self.is_valid(&agent_id) {
            bail!("agent {} is bound more than once", agent_id);
        }
        self.items.push(agent_id);
        Ok(())
    }

    pub fn pop(&mut self) {
        self.items.pop();
    }
}

impl Default for ValidAgents {
    fn default() -> Self {
        ValidAgents { items: vec![] }
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
}

impl ResultType {
    pub fn empty_boolean() -> Self {
        ResultType::Boolean {
            true_guarantees: Guarantees::default(),
            false_guarantees: Guarantees::default(),
        }
    }
}

pub fn type_check(expression: &Expression, world: &World) -> Result<ResultType> {
    let world_entity_ids: Vec<String> = world
        .iter_agents()
        .map(|(agent_id, _)| agent_id.clone())
        .collect();
    type_check_helper(
        expression,
        world,
        &mut Guarantees::default(),
        &mut ValidAgents::new(world_entity_ids),
    )
}

fn type_check_helper(
    expression: &Expression,
    world: &World,
    guarantees: &mut Guarantees,
    valid_agents: &mut ValidAgents,
) -> Result<ResultType> {
    match expression {
        Expression::Value(value) => match value {
            Value::Number(_) => Ok(ResultType::Number),
            Value::Boolean(_) => Ok(ResultType::empty_boolean()),
            Value::Variant(_) => Ok(ResultType::Variant),
            Value::String(_) => Ok(ResultType::String),
            Value::Variable(sentence) => type_check_query(
                Query::parse(world, sentence, &Bindings::default())?,
                world,
                guarantees,
                valid_agents,
            ),
        },
        Expression::And(x, y) => {
            let x = type_check_helper(x, world, guarantees, valid_agents)?;
            let ResultType::Boolean {
                true_guarantees: x_true_guarantees,
                ..
            } = x
            else {
                bail!("And conditions must be boolean, got {:?}", x);
            };

            // We can evaluate the second condition under the assumption that
            // the first condition is true because of short-circuiting! It will
            // not be run otherwise!
            let added_count = guarantees.push_many(x_true_guarantees.items.clone());
            let y = type_check_helper(y, world, guarantees, valid_agents)?;
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
                other => bail!("And conditions must be boolean, got {:?}", other),
            }
        }
        Expression::Or(x, y) => {
            let x = type_check_helper(x, world, guarantees, valid_agents)?;
            let ResultType::Boolean {
                false_guarantees: x_false_guarantees,
                ..
            } = x
            else {
                bail!("Or conditions must be boolean, got {:?}", x);
            };

            // We can evaluate the second condition under the assumption that
            // the first condition is false because of short-circuiting! It
            // will not be run otherwise!
            let added_count = guarantees.push_many(x_false_guarantees.items.clone());
            let y = type_check_helper(y, world, guarantees, valid_agents)?;
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
                other => bail!("Or conditions must be boolean, got {:?}", other),
            }
        }
        Expression::Is(x, y) => {
            let x = type_check_helper(x, world, guarantees, valid_agents)?;
            let y = type_check_helper(y, world, guarantees, valid_agents)?;
            // The only condition is that they are the same type
            if std::mem::discriminant(&x) != std::mem::discriminant(&y) {
                bail!(
                    "Is conditions must compare values of the same type, got {:?} and {:?}",
                    x,
                    y
                );
            }
            // Can theoretically introduce dependent guarantees, but that's too
            // complicated for my tired brain right now.
            Ok(ResultType::empty_boolean())
        }
        Expression::Not(expr) => {
            let res = type_check_helper(expr, world, guarantees, valid_agents)?;
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
                other => bail!("Not conditions must be boolean, got {:?}", other),
            }
        }
        Expression::ForAll(new_symbol, expression) => {
            // The new symbol represents a new valid agent binding
            valid_agents.push(new_symbol.clone())?;
            let res = type_check_helper(expression, world, guarantees, valid_agents)?;
            valid_agents.pop();
            match res {
                ResultType::Boolean { .. } => Ok(ResultType::empty_boolean()),
                other => bail!("ForAll conditions must be boolean, got {:?}", other),
            }
        }
        Expression::Any(new_symbol, expression) => {
            // The new symbol represents a new valid agent binding
            valid_agents.push(new_symbol.clone())?;
            let res = type_check_helper(expression, world, guarantees, valid_agents)?;
            valid_agents.pop();
            match res {
                ResultType::Boolean { .. } => Ok(ResultType::empty_boolean()),
                other => bail!("Any conditions must be boolean, got {:?}", other),
            }
        }
        Expression::Count(new_symbol, expression) => {
            // The new symbol represents a new valid agent binding
            valid_agents.push(new_symbol.clone())?;
            let res = type_check_helper(expression, world, guarantees, valid_agents)?;
            valid_agents.pop();
            match res {
                ResultType::Boolean { .. } => Ok(ResultType::empty_boolean()),
                other => bail!("Any conditions must be boolean, got {:?}", other),
            }
        }
        Expression::Aggregate {
            body, var, filter, ..
        } => {
            // The new symbol represents a new valid agent binding
            valid_agents.push(var.clone())?;
            let filter_res = type_check_helper(expression, world, guarantees, valid_agents)?;
            valid_agents.pop();

            let ResultType::Boolean {
                true_guarantees, ..
            } = filter_res
            else {
                bail!("Aggregate filter must be boolean, got {:?}", filter);
            };

            // We can evaluate the aggregate body under the assumptions of the
            // filter because any value being evaluated has passed the filter.
            // Push valid agent again here to keep these atomic.
            valid_agents.push(var.clone())?;
            let added_count = guarantees.push_many(true_guarantees.items.clone());
            let body_res = type_check_helper(body, world, guarantees, valid_agents)?;
            valid_agents.pop();
            guarantees.pop_many(added_count);

            match body_res {
                ResultType::Number => Ok(ResultType::Number),
                other => bail!("Aggregate bodies must be numeric, got {:?}", other),
            }
        }
    }
}

fn type_check_query(
    query: Query,
    world: &World,
    guarantees: &Guarantees,
    valid_agents: &ValidAgents,
) -> Result<ResultType> {
    match query {
        Query::Fielded(relation_query, field_name) => {
            valid_agents.validate_query(&relation_query)?;

            // Fielded relationships require a guarantee because the lookup is
            // not possible with a nonexistent relationship.
            if !guarantees.is_guaranteed(&relation_query) {
                bail!(
                    "cannot access field {} of {:?} without guarantee that it exists",
                    field_name,
                    relation_query
                );
            }

            let relation_type = world
                .get_type_mapping()
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
            }
        }
        Query::Unfielded(relation_query) => {
            valid_agents.validate_query(&relation_query)?;

            // Unfielded queries always return booleans, and we can introduce
            // a true-guarantee for it because it acts as an existence check.
            Ok(ResultType::Boolean {
                true_guarantees: Guarantees::new(vec![relation_query]),
                false_guarantees: Guarantees::default(),
            })
        }
    }
}
