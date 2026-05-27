use std::{collections::HashMap, fmt};

use anyhow::{Context, Result, bail};

use crate::{
    anyhow_ext::ResultOptionExt,
    expressions::Expression,
    queries::{ActorRef, Query, RelationQuery},
    types::{FieldType, RelationTypeData},
    values::{Constant, Sentence, Value},
    world::{
        Bindings, Relation, RelationData, RelationHandle, World,
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
    Increase(Sentence, f64),
    Cycle(Sentence, f64),
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
        .with_context(|| format!("processing declaration {}", declaration.sentence))?;

    // TODO: relations with one parameter should be initializable this way!
    let Query::Unfielded(relation_query) = &query else {
        bail!("extra parameters in declaration {}", declaration.sentence);
    };

    let relation_query = relation_query.apply_bindings(bindings);

    match relation_query {
        RelationQuery::Trait { actor, trait_name } => {
            world.add_trait(actor.as_literal()?, &trait_name, declaration.fields.clone())
        }
        RelationQuery::Emotion {
            actor,
            emotion_name,
        } => world.add_emotion(
            actor.as_literal()?,
            &emotion_name,
            declaration.fields.clone(),
        ),
        RelationQuery::Binary {
            actor_1,
            actor_2,
            type_name,
        } => world.add_binary_relation(
            actor_1.as_literal()?,
            actor_2.as_literal()?,
            &type_name,
            declaration.fields.clone(),
        ),
        RelationQuery::Practice {
            participants,
            type_name,
        } => world.add_practice(
            participants
                .iter()
                .map(ActorRef::as_literal)
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
                "evaluating variable with sentence {} and bindings {:?}",
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
            "condition expression must evaluate to boolean, got {}",
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
        .with_context(|| format!("processing delete outcome {}", sentence))?;

    let Query::Unfielded(relation_query) = &query else {
        bail!("extra parameters in delete outcome {}", sentence);
    };

    let relation_query = relation_query.apply_bindings(bindings);

    let (edge, _) = relation_query
        .lookup_in_world(world.inner())
        .require_with_context(|| format!("relation not found in delete outcome {}", sentence))?;
    world
        .remove_relation(edge.relation_handle.clone())
        .with_context(|| format!("removing relation in delete outcome {}", sentence))
}

fn process_update(
    world: &mut WorldTxn,
    sentence: &Sentence,
    value: &Value,
    bindings: &Bindings,
) -> Result<()> {
    let query = Query::parse(world.inner(), sentence, bindings)
        .with_context(|| format!("processing update outcome {}", sentence))?;
    let Query::Fielded(relation_query, field_name) = &query else {
        bail!(
            "update outcome must specify a field to update: {}",
            sentence
        );
    };

    let relation_query = relation_query.apply_bindings(bindings);

    let (edge, _) = relation_query
        .lookup_in_world(world.inner())
        .require_with_context(|| format!("relation not found in update outcome {}", sentence))?;

    let constant_value = match value {
        Value::Number(n) => Constant::Number(*n),
        Value::Boolean(b) => Constant::Boolean(*b),
        Value::Variant(v) => Constant::Variant(v.clone()),
        Value::String(s) => Constant::String(s.clone()),
        Value::ActorRef(_) => todo!(),
        Value::Variable(new_val_sentence) => {
            evaluate_variable(world.inner(), new_val_sentence, bindings).with_context(|| {
                format!(
                    "evaluating variable for new value in update outcome with sentence {}",
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
        .with_context(|| format!("applying update outcome {}", sentence))
}

fn get_value_and_field_type<'a, 'b>(
    world: &'a World,
    relation: &'b Relation,
    field_name: &str,
) -> Result<(&'b Constant, &'a FieldType)> {
    let current_value = relation
        .fields
        .get(field_name)
        .with_context(|| format!("field {} not found in relation", field_name))?;

    let Some(relation_type) = world.get_relation_type_map().get_type(&relation.type_name) else {
        bail!("relation type {} not found", relation.type_name);
    };

    let Some(field_type) = relation_type.fields.get(field_name) else {
        bail!(
            "field {} not found in relation type {}",
            field_name,
            &relation_type.name
        );
    };

    Ok((current_value, field_type))
}

fn process_increase(
    world: &mut WorldTxn,
    sentence: &Sentence,
    amount: f64,
    bindings: &Bindings,
) -> Result<()> {
    let query = Query::parse(world.inner(), sentence, bindings)
        .with_context(|| format!("processing increase outcome {}", sentence))?;
    let Query::Fielded(relation_query, field_name) = &query else {
        bail!(
            "increase outcome must specify a field to increase: {}",
            sentence
        );
    };

    let relation_query = relation_query.apply_bindings(bindings);

    let (edge, relation) = relation_query
        .lookup_in_world(world.inner())
        .require_with_context(|| format!("relation not found in increase outcome {}", sentence))?;

    let (current_value, field_type) = get_value_and_field_type(world.inner(), relation, field_name)
        .with_context(|| {
            format!(
                "getting current value and field type for increase outcome {}",
                sentence
            )
        })?;

    match (current_value, field_type) {
        (Constant::Number(current), FieldType::NumberRange(low, high)) => {
            let new_value = (current + (amount as f64)).clamp(*low, *high);
            world
                .update_relation(
                    edge.relation_handle.clone(),
                    HashMap::from([(field_name.clone(), Constant::Number(new_value))]),
                )
                .with_context(|| format!("applying increase outcome {}", sentence))
        }
        (Constant::Variant(current), FieldType::VariantList(variants)) => {
            let current_index = variants
                .iter()
                .position(|v| v == current)
                .with_context(|| {
                    format!(
                        "current variant value {} not found in variants list for increase outcome {}",
                        current, sentence
                    )
                })?;
            let amount = amount.round() as i64;
            let new_index =
                (current_index as i64 + amount).clamp(0, (variants.len() - 1) as i64) as usize;
            world
                .update_relation(
                    edge.relation_handle.clone(),
                    HashMap::from([(
                        field_name.clone(),
                        Constant::Variant(variants[new_index].clone()),
                    )]),
                )
                .with_context(|| format!("applying increase outcome {}", sentence))
        }
        // Can also fail if the field type is wrong, but that should never happen.
        // Just worth noting.
        _ => bail!(
            "increase outcome only applies to number ranges and variants, found {}: {}",
            field_type,
            sentence
        ),
    }
}

fn process_cycle(
    world: &mut WorldTxn,
    sentence: &Sentence,
    amount: f64,
    bindings: &Bindings,
) -> Result<()> {
    let query = Query::parse(world.inner(), sentence, bindings)
        .with_context(|| format!("processing cycle outcome {}", sentence))?;
    let Query::Fielded(relation_query, field_name) = &query else {
        bail!("cycle outcome must specify a field to cycle: {}", sentence);
    };

    let relation_query = relation_query.apply_bindings(bindings);

    let (edge, relation) = relation_query
        .lookup_in_world(world.inner())
        .require_with_context(|| format!("relation not found in cycle outcome {}", sentence))?;

    let (current_value, field_type) = get_value_and_field_type(world.inner(), relation, field_name)
        .with_context(|| {
            format!(
                "getting current value and field type for cycle outcome {}",
                sentence
            )
        })?;

    match (current_value, field_type) {
        (Constant::Number(current), FieldType::NumberRange(low, high)) => {
            let range = high - low;
            let new_value = ((current - low + amount).rem_euclid(range)) + low;
            world
                .update_relation(
                    edge.relation_handle.clone(),
                    HashMap::from([(field_name.clone(), Constant::Number(new_value))]),
                )
                .with_context(|| format!("applying cycle outcome {}", sentence))
        }
        (Constant::Variant(current), FieldType::VariantList(variants)) => {
            let current_index = variants
                .iter()
                .position(|v| v == current)
                .with_context(|| {
                    format!(
                        "current variant value {} not found in variants list for cycle outcome {}",
                        current, sentence
                    )
                })?;
            let amount = amount.round() as i64;
            let new_index =
                ((current_index as i64 + amount).rem_euclid(variants.len() as i64)) as usize;
            world
                .update_relation(
                    edge.relation_handle.clone(),
                    HashMap::from([(
                        field_name.clone(),
                        Constant::Variant(variants[new_index].clone()),
                    )]),
                )
                .with_context(|| format!("applying cycle outcome {}", sentence))
        }
        // Can also fail if the field type is wrong, but that should never happen.
        // Just worth noting.
        _ => bail!(
            "cycle outcome only applies to number ranges and variants, found {}: {}",
            field_type,
            sentence
        ),
    }
}

pub fn process_effect(
    world: &mut WorldTxn,
    actor_name: &str,
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
                Some(actor_name),
                string,
                bindings,
            )?));
        }
        Effect::Activate(actor_id) => world.set_actor_active(&bindings.get_or_same(actor_id), true),
        Effect::Deactivate(actor_id) => {
            world.set_actor_active(&bindings.get_or_same(actor_id), false)
        }
        Effect::Delete(sentence) => process_delete(world, sentence, bindings),
        Effect::Set(declaration) => process_declaration(world, declaration, bindings).map(|_| ()),
        Effect::Update(sentence, value) => process_update(world, sentence, value, bindings),
        Effect::Increase(sentence, amount) => process_increase(world, sentence, *amount, bindings),
        Effect::Cycle(sentence, amount) => process_cycle(world, sentence, *amount, bindings),
    }?;
    Ok(None)
}

pub fn get_available_actions(world: &World, actor_id: &str) -> Result<Vec<ActionRef>> {
    let actor = world
        .get_actor(actor_id)
        .with_context(|| format!("actor {} not found", actor_id))?;
    let mut available_actions = Vec::new();

    for (edge, relation) in world.iter_actor_relations(actor) {
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
                    if action_for != actor_id {
                        continue;
                    }
                    for condition in &action.conditions {
                        if !check_condition(world, condition, bindings).with_context(|| {
                            format!(
                                "checking conditions for action {} of practice {}",
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
                                    "formatting display name for action {} of practice {}",
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
/// as part of a net delta of an actor's goals across two world states. The
/// return value is derived from the goal's measurements and weights.
///
/// This system supports the same unbound variables as conditions do, and
/// any expressions with multiple possible bindings will have their weights
/// summed.
///
/// WARNING: If a new edge is added that gets caught by an increase
/// measurement, it will result in a huge delta for that event. I would
/// recommend normalizing your values for this; throwing away non-delta
/// scores would fix this issue, but also throws away pocurrent value and field type do not match for increase outcometentially useful
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
                "goal expression must evaluate to boolean for Exists measurement, got {}",
                other
            ),
        },
        GoalMeasurement::Delta => match evaluation {
            Constant::Number(n) => {
                total_weight += n * goal.weight;
            }
            other => bail!(
                "goal expression must evaluate to number for Increase/Decrease measurement, got {}",
                other
            ),
        },
    }

    Ok(total_weight)
}

/// Evaluates all of an actor's goals and returns the total score. This is
/// intended to be used as part of a net delta of an actor's goals across
/// two world states, so the return value is not entirely useful by itself.
/// The return value is derived from the goals' measurements and weights.
pub fn evaluate_actor_goals(world: &World, actor_id: &str) -> Result<f64> {
    let actor = world
        .get_actor(actor_id)
        .with_context(|| format!("actor {} not found for goal evaluation", actor_id))?;
    let mut total_score = 0.0;
    for goal in &actor.goals {
        total_score += evaluate_goal(world, goal, &Bindings::default())?;
    }
    Ok(total_score)
}
