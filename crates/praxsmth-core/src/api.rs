use anyhow::{Context, Result};

use crate::{
    parser::{
        parse_effect_str, parse_expression_str,
        types::parse_types,
        world::{WorldInitStep, parse_world},
    },
    types::{RelationTypeMap, checking::type_check_world},
    values::Constant,
    world::{
        Relation, RelationHandle, World,
        bindings::Bindings,
        simulation::{
            ActionRef, Dialog, evaluate_actor_goals, get_available_actions,
            process_available_action, process_declaration, process_effect,
        },
        transactions::WorldTxn,
    },
};

pub struct ActorInfo {
    pub id: String,
    pub name: String,
    pub active: bool,
}

pub struct RelationInfo {
    pub type_id: String,
    pub actors: Vec<String>,
    pub fields: Vec<(String, Constant)>,
    pub sentence: String,
}

pub struct AvailableAction {
    pub index: usize,
    pub display_name: String,
    pub goal_delta: f64,
}

/// The main API for interacting with the Praxsmth world. This is the intended
/// interface for external code to use when working with the world. It combines
/// both the `World` and `Simulation` into a single struct, and provides
/// convenience methods for common operations like parsing from strings,
/// getting available actions, and applying actions.
pub struct PraxsmthApi {
    pub dialog_history: Vec<Dialog>,
    world: World,
}

impl PraxsmthApi {
    pub fn new(world: World) -> Self {
        Self {
            dialog_history: Vec::new(),
            world,
        }
    }

    /// Parse a world from strings containing the type definitions and world definitions.
    pub fn from_strings(types: &str, world: &str) -> Result<Self> {
        let type_defs = parse_types(types).context("parsing types")?;
        let world_defs = parse_world(world).context("parsing world")?;

        let type_mapping =
            RelationTypeMap::from_types(type_defs).context("constructing type mapping")?;
        let mut world = World::new(type_mapping);

        let empty_bindings = Bindings::default();

        for world_def in world_defs.into_iter() {
            match world_def {
                WorldInitStep::NewActor(actor_info) => {
                    world
                        .add_actor(&actor_info)
                        .with_context(|| format!("adding actor {}", actor_info.name))?;
                }
                WorldInitStep::NewRelation(declaration) => {
                    let mut transaction = world.transaction();
                    process_declaration(&mut transaction, &declaration, &empty_bindings)
                        .with_context(|| {
                            format!("processing declaration {:?}", declaration.sentence)
                        })?;
                    transaction.commit();
                }
            }
        }

        type_check_world(&world).with_context(|| "type checking world after initialization")?;

        Ok(Self::new(world))
    }

    /// Parse and apply a single effect (e.g. `set actor.likes.food { amount: 5 }`) to the
    /// world on behalf of `actor_name`, committing the change. Returns any dialog the
    /// effect produced (e.g. from `say`/`broadcast`).
    pub fn process_effect(&mut self, actor_name: &str, input: &str) -> Result<Option<Dialog>> {
        let effect = parse_effect_str(input).context("parsing effect")?;
        let bindings = Bindings::default();

        let mut transaction = self.world.transaction();
        let dialog = process_effect(&mut transaction, actor_name, &effect, &bindings)
            .with_context(|| format!("applying effect {:?}", effect))?;
        transaction.commit();

        Ok(dialog)
    }

    /// Parse and evaluate a single expression (e.g. `a is b and not c`) against the
    /// current world state, returning the resulting constant.
    pub fn evaluate_expression(&self, input: &str) -> Result<Constant> {
        let expression = parse_expression_str(input).context("parsing expression")?;
        let bindings = Bindings::default();

        expression
            .evaluate(&self.world, &bindings)
            .with_context(|| format!("evaluating expression {:?}", expression))
    }

    /// Get the names of the available actions for an actor.
    /// The order for this is deterministic, so that the same action will always have the same index.
    pub fn get_available_actions(
        &mut self,
        actor_id: &str,
        score_depth: usize,
    ) -> Result<Vec<AvailableAction>> {
        let actions = get_available_actions(&self.world, actor_id)
            .with_context(|| format!("getting available action names for actor {}", actor_id))?;

        let base_score = evaluate_actor_goals(&self.world, actor_id).with_context(|| {
            format!(
                "evaluating goals for actor {} before applying actions",
                actor_id
            )
        })?;

        let mut available_actions = vec![];

        for (i, action) in actions.iter().enumerate() {
            let mut transaction = self.world.transaction();
            let score = Self::score_action(&mut transaction, actor_id, action, score_depth)
                .with_context(|| format!("scoring action {:?} for actor {}", action, actor_id))?;
            available_actions.push(AvailableAction {
                index: i,
                display_name: action.display_name.clone(),
                goal_delta: score - base_score,
            });
        }

        Ok(available_actions)
    }

    fn score_action(
        world: &mut WorldTxn,
        actor_id: &str,
        action: &ActionRef,
        depth: usize,
    ) -> Result<f64> {
        if depth == 0 {
            return Ok(0.0);
        }

        let savepoint = world.savepoint();

        process_available_action(world, action)?;

        let score = if depth == 1 {
            evaluate_actor_goals(world.inner(), actor_id)?
        } else {
            let actions = get_available_actions(world.inner(), actor_id)?;

            if actions.is_empty() {
                // This tree ends here, so return the score of the current state.
                evaluate_actor_goals(world.inner(), actor_id)?
            } else {
                let mut best_score = f64::NEG_INFINITY;
                for action in actions {
                    let action_score = Self::score_action(world, actor_id, &action, depth - 1)?;
                    if action_score > best_score {
                        best_score = action_score;
                    }
                }
                best_score
            }
        };

        world.rollback_to(savepoint)?;

        Ok(score)
    }

    /// Apply an action by its index in the list of available actions for an actor.
    pub fn apply_action(&mut self, actor_id: &str, action_index: u32) -> Result<Vec<Dialog>> {
        let actions = get_available_actions(&self.world, actor_id).with_context(|| {
            format!(
                "getting available actions for actor {} before apply",
                actor_id
            )
        })?;
        let action = actions.get(action_index as usize).with_context(|| {
            format!(
                "action index {} out of bounds for actor {} (have {} actions)",
                action_index,
                actor_id,
                actions.len()
            )
        })?;

        let mut transaction = self.world.transaction();
        let dialog = process_available_action(&mut transaction, action)
            .with_context(|| format!("applying action {} for actor {}", action_index, actor_id))?;
        transaction.commit();
        self.dialog_history.extend(dialog.clone());
        Ok(dialog)
    }

    /// Gets the current emotion of the actor, if any.
    pub fn get_current_emotion(
        &self,
        actor_id: &str,
    ) -> Result<Option<(RelationHandle, &Relation)>> {
        Ok(self
            .world
            .get_actor(actor_id)
            .with_context(|| format!("could not find actor {} in world", actor_id))?
            .emotion
            .as_ref()
            .and_then(|handle| {
                self.world
                    .get_relation(handle.clone())
                    .map(|relation| (handle.clone(), relation))
            }))
    }

    /// Gets the info of every actor in the world.
    pub fn get_actor_info(&self) -> Vec<ActorInfo> {
        self.world
            .iter_actors()
            .map(|(id, actor)| ActorInfo {
                id: id.clone(),
                name: actor.name.clone(),
                active: actor.is_active,
            })
            .collect()
    }

    /// Gets the info of every relation in the world.
    pub fn get_relation_info(&self) -> Vec<RelationInfo> {
        self.world
            .iter_relations()
            .map(|(_, relation)| RelationInfo {
                type_id: relation.type_name.clone(),
                actors: relation.iter_actor_ids().map(|id| id.to_string()).collect(),
                fields: relation
                    .fields
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect(),
                sentence: relation.sentence.to_string(),
            })
            .collect()
    }
}
