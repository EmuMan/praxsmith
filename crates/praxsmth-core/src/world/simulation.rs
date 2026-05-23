use std::{collections::HashMap, fmt};

use anyhow::{Context, Result, bail};

use crate::{
    anyhow_ext::ResultOptionExt,
    expressions::Expression,
    queries::{AgentRef, Query, RelationQuery},
    types::RelationTypeData,
    values::{Constant, Sentence, Value},
    world::{
        Bindings, RelationData, RelationHandle, World,
        goals::{Goal, GoalMeasurement},
        transactions::WorldTxn,
    },
};

#[derive(Debug, Clone)]
pub struct ActionRef {
    pub display_name: String,
    pub overall_index: usize,
    pub practice_handle: RelationHandle,
    pub index_within_practice: usize,
}

#[derive(Debug, Clone)]
pub struct Dialog {
    pub speaker: Option<String>,
    pub line: String,
}

#[derive(Debug, Clone)]
pub struct Declaration {
    pub sentence: Sentence,
    pub fields: HashMap<String, Constant>,
}

impl fmt::Display for Declaration {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fields_str: Vec<_> = self
            .fields
            .iter()
            .map(|(name, value)| format!("{}: {}", name, value))
            .collect();
        write!(
            f,
            "declaration {} {{{}}}",
            self.sentence,
            fields_str.join(", ")
        )
    }
}

#[derive(Debug, Clone)]
pub enum Effect {
    Broadcast(String),
    Say(String),
    Activate(String),
    Deactivate(String),
    Delete(Sentence),
    Set(Declaration),
    Update(Sentence, Value),
    Increase(Sentence, i64),
    Cycle(Sentence, i64),
}

/// Adds the information contained within a declaration to the world state.
///
/// The sentence within the declaration must match a query. An error will
/// be raised if there are any free variables within this query.
pub fn process_declaration(
    world: &mut WorldTxn,
    declaration: &Declaration,
    bindings: &Bindings,
) -> Result<RelationHandle> {
    let query = Query::parse(world.inner(), &declaration.sentence, bindings)
        .with_context(|| format!("processing declaration {:?}", declaration.sentence))?;

    // TODO: relations with one parameter should be initializable this way!
    let Query::Unfielded(relation_query) = &query else {
        bail!("extra parameters in declaration {:?}", declaration.sentence);
    };

    let relation_query = relation_query.apply_bindings(bindings);

    match relation_query {
        RelationQuery::Trait { agent, trait_name } => {
            world.add_trait(agent.as_literal()?, &trait_name, declaration.fields.clone())
        }
        RelationQuery::Emotion {
            agent,
            emotion_name,
        } => world.add_emotion(
            agent.as_literal()?,
            &emotion_name,
            declaration.fields.clone(),
        ),
        RelationQuery::Binary {
            agent_1,
            agent_2,
            type_name,
        } => world.add_binary_relation(
            agent_1.as_literal()?,
            agent_2.as_literal()?,
            &type_name,
            declaration.fields.clone(),
        ),
        RelationQuery::Practice {
            participants,
            type_name,
        } => world.add_practice(
            participants
                .iter()
                .map(AgentRef::as_literal)
                .collect::<Result<Vec<&str>>>()?,
            &type_name,
            declaration.fields.clone(),
        ),
    }
}

/// Evaluates a variable (i.e. a sentence) to a constant value. The
/// sentence must be parsable into a relation query. Bindings will be
/// applied to the query before evaluation, so free variables can be used
/// in the sentence as long as they are bound in the provided bindings.
/// Returns an error if the sentence cannot be parsed into a relation
/// if there are any free variables in the query after bindings are
/// applied, or if a fielded query specifies a relation that does not exist
/// in the world.
pub fn evaluate_variable(
    world: &World,
    sentence: &Sentence,
    bindings: &Bindings,
) -> Result<Constant> {
    Query::parse(world, sentence, bindings)?
        .apply_bindings(bindings)
        .evaluate_in_world(world)
        .with_context(|| {
            format!(
                "evaluating variable with sentence {:?} and bindings {:?}",
                sentence, bindings
            )
        })
}

pub fn check_condition(
    world: &World,
    expression: &Expression,
    bindings: &Bindings,
) -> Result<bool> {
    match expression.evaluate(world, bindings)? {
        Constant::Boolean(b) => Ok(b),
        other => bail!(
            "condition expression must evaluate to boolean, got {:?}",
            other
        ),
    }
}

fn process_print(
    world: &World,
    speaker: Option<&str>,
    string: &str,
    bindings: &Bindings,
) -> Result<Dialog> {
    let dialog = Dialog {
        speaker: speaker.map(|s| s.to_string()),
        line: world.format_string(string, bindings).with_context(|| {
            format!(
                "formatting string for print outcome with speaker {:?}: {}",
                speaker, string
            )
        })?,
    };
    Ok(dialog)
}

fn process_delete(world: &mut WorldTxn, sentence: &Sentence, bindings: &Bindings) -> Result<()> {
    let query = Query::parse(world.inner(), sentence, bindings)
        .with_context(|| format!("processing delete outcome {:?}", sentence))?;

    let Query::Unfielded(relation_query) = &query else {
        bail!("extra parameters in delete outcome {:?}", sentence);
    };

    let relation_query = relation_query.apply_bindings(bindings);

    let (edge, _) = relation_query
        .lookup_in_world(world.inner())
        .require_with_context(|| format!("relation not found in delete outcome {:?}", sentence))?;
    world
        .remove_relation(edge.relation_handle.clone())
        .with_context(|| format!("removing relation in delete outcome {:?}", sentence))
}

fn process_update(
    world: &mut WorldTxn,
    sentence: &Sentence,
    value: &Value,
    bindings: &Bindings,
) -> Result<()> {
    let query = Query::parse(world.inner(), sentence, bindings)
        .with_context(|| format!("processing update outcome {:?}", sentence))?;
    let Query::Fielded(relation_query, field_name) = &query else {
        // TODO: Support bracket syntax so this is actually usable!!!
        // (Currently you can't initialize anything with >1 fields)
        // Just needs to be added to parser and the logic flow here,
        // the update functions should already support it.
        bail!(
            "update outcome must specify a field to update {:?}",
            sentence
        );
    };

    let relation_query = relation_query.apply_bindings(bindings);

    let (edge, _) = relation_query
        .lookup_in_world(world.inner())
        .require_with_context(|| format!("relation not found in update outcome {:?}", sentence))?;

    let constant_value = match value {
        Value::Number(n) => Constant::Number(*n),
        Value::Boolean(b) => Constant::Boolean(*b),
        Value::Variant(v) => Constant::Variant(v.clone()),
        Value::String(s) => Constant::String(s.clone()),
        Value::Variable(new_val_sentence) => {
            evaluate_variable(world.inner(), new_val_sentence, bindings).with_context(|| {
                format!(
                    "evaluating variable for new value in update outcome with sentence {:?}",
                    new_val_sentence
                )
            })?
        }
    };

    world
        .update_relation(
            edge.relation_handle.clone(),
            HashMap::from([(field_name.clone(), constant_value)]),
        )
        .with_context(|| format!("applying update outcome {:?}", sentence))
}

fn process_increase(
    _world: &mut WorldTxn,
    _sentence: &Sentence,
    _amount: i64,
    _bindings: &Bindings,
) -> Result<()> {
    unimplemented!()
}

fn process_cycle(
    _world: &mut WorldTxn,
    _sentence: &Sentence,
    _amount: i64,
    _bindings: &Bindings,
) -> Result<()> {
    unimplemented!()
}

pub fn process_effect(
    world: &mut WorldTxn,
    agent_name: &str,
    effect: &Effect,
    bindings: &Bindings,
) -> Result<Option<Dialog>> {
    match effect {
        Effect::Broadcast(string) => {
            return Ok(Some(process_print(world.inner(), None, string, bindings)?));
        }
        Effect::Say(string) => {
            return Ok(Some(process_print(
                world.inner(),
                Some(agent_name),
                string,
                bindings,
            )?));
        }
        Effect::Activate(agent_id) => world.set_agent_active(&bindings.get_or_same(agent_id), true),
        Effect::Deactivate(agent_id) => {
            world.set_agent_active(&bindings.get_or_same(agent_id), false)
        }
        Effect::Delete(sentence) => process_delete(world, sentence, bindings),
        Effect::Set(declaration) => process_declaration(world, declaration, bindings).map(|_| ()),
        Effect::Update(sentence, value) => process_update(world, sentence, value, bindings),
        Effect::Increase(sentence, amount) => process_increase(world, sentence, *amount, bindings),
        Effect::Cycle(sentence, amount) => process_cycle(world, sentence, *amount, bindings),
    }?;
    Ok(None)
}

pub fn get_available_actions(world: &World, agent_name: &str) -> Result<Vec<ActionRef>> {
    let agent = world
        .get_agent(agent_name)
        .with_context(|| format!("agent {} not found", agent_name))?;
    let mut available_actions = Vec::new();

    for (edge, relation) in world.iter_agent_relations(agent) {
        match &relation.data {
            RelationData::Practice { bindings, .. } => {
                let relation_type = world
                    .relation_type_map
                    .get_type(&relation.type_name)
                    .with_context(|| {
                        format!("type {} not found for practice action", relation.type_name)
                    })?;
                let RelationTypeData::Practice { actions, .. } = &relation_type.data else {
                    bail!(
                        "type {} data is not practice for action lookup",
                        relation.type_name
                    );
                };
                'action_loop: for (i, action) in actions.iter().enumerate() {
                    let action_for = World::resolve_binding_or_same(&action.for_actor, bindings);
                    if action_for != agent_name {
                        continue;
                    }
                    for condition in &action.conditions {
                        if !check_condition(world, condition, bindings).with_context(|| {
                            format!(
                                "checking conditions for action {} of practice {:?}",
                                action.name, relation_type.name
                            )
                        })? {
                            continue 'action_loop;
                        }
                    }

                    available_actions.push(ActionRef {
                        display_name: world.format_string(&action.name, bindings).with_context(
                            || {
                                format!(
                                    "formatting display name for action {} of practice {:?}",
                                    action.name, relation_type.name
                                )
                            },
                        )?,
                        overall_index: available_actions.len(),
                        practice_handle: edge.relation_handle.clone(),
                        index_within_practice: i,
                    });
                }
            }
            _ => {}
        }
    }

    Ok(available_actions)
}

pub fn process_available_action(
    world: &mut WorldTxn,
    available_action: &ActionRef,
) -> Result<Vec<Dialog>> {
    let relation = world
        .inner()
        .get_relation(available_action.practice_handle.clone())
        .with_context(|| {
            format!(
                "relation {:?} not found for available action",
                available_action.practice_handle
            )
        })?;
    let RelationData::Practice { bindings, .. } = &relation.data else {
        bail!(
            "relation {:?} data is not practice for available action",
            available_action.practice_handle
        );
    };
    let relation_type = world
        .inner()
        .relation_type_map
        .get_type(&relation.type_name)
        .with_context(|| format!("type {} not found for available action", relation.type_name))?;
    let RelationTypeData::Practice { actions, .. } = &relation_type.data else {
        bail!(
            "type {} is not a practice for available action",
            relation.type_name
        );
    };
    let action = actions
        .get(available_action.index_within_practice)
        .with_context(|| {
            format!(
                "action index {} out of bounds for practice {:?}",
                available_action.index_within_practice, available_action.practice_handle
            )
        })?;

    // TODO: Fix this a better way.
    let actor_name = World::resolve_binding_or_same(&action.for_actor, &bindings);
    let effects = action.effects.clone();
    let action_name = action.name.clone();
    let bindings = bindings.clone();

    let mut dialog: Vec<Dialog> = vec![];

    for effect in &effects {
        if let Some(new_dialog) = process_effect(world, &actor_name, effect, &bindings)
            .with_context(|| format!("processing effect of action {}", action_name))?
        {
            dialog.push(new_dialog);
        }
    }

    Ok(dialog)
}

/// Evaluates the current state of a goal within the context of a world.
/// This value is not entirely useful by itself, and is intended to be used
/// as part of a net delta of an agent's goals across two world states. The
/// return value is derived from the goal's measurements and weights.
///
/// This system supports the same unbound variables as conditions do, and
/// any expressions with multiple possible bindings will have their weights
/// summed.
///
/// WARNING: If a new edge is added that gets caught by an increase
/// measurement, it will result in a huge delta for that event. I would
/// recommend normalizing your values for this; throwing away non-delta
/// scores would fix this issue, but also throws away potentially useful
/// information, so I've decided not to do that.
fn evaluate_goal(world: &World, goal: &Goal, bindings: &Bindings) -> Result<f64> {
    let evaluation = goal.expression.evaluate(world, bindings)?;

    let mut total_weight = 0.0;
    match goal.measurement {
        GoalMeasurement::Exists => match evaluation {
            Constant::Boolean(b) => {
                if b {
                    total_weight += goal.weight;
                }
            }
            other => bail!(
                "goal expression must evaluate to boolean for Exists measurement, got {:?}",
                other
            ),
        },
        GoalMeasurement::Delta => match evaluation {
            Constant::Number(n) => {
                total_weight += n * goal.weight;
            }
            other => bail!(
                "goal expression must evaluate to number for Increase/Decrease measurement, got {:?}",
                other
            ),
        },
    }

    Ok(total_weight)
}

/// Evaluates all of an agent's goals and returns the total score. This is
/// intended to be used as part of a net delta of an agent's goals across
/// two world states, so the return value is not entirely useful by itself.
/// The return value is derived from the goals' measurements and weights.
pub fn evaluate_agent_goals(world: &World, agent_id: &str) -> Result<f64> {
    let agent = world
        .get_agent(agent_id)
        .with_context(|| format!("agent {} not found for goal evaluation", agent_id))?;
    let mut total_score = 0.0;
    for goal in &agent.goals {
        total_score += evaluate_goal(world, goal, &Bindings::default())?;
    }
    Ok(total_score)
}
