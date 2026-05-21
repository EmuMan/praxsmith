use anyhow::{Context, Result};

use crate::{
    definitions::{PraxsmthConstant, world::PraxsmthWorldDefinition},
    parser::{parse_effect_str, parse_expression_str, types::parse_types, world::parse_world},
    types::TypeMapping,
    world::{
        Bindings, Relation, RelationHandle, World,
        simulation::{ActionRef, Dialog, Simulation},
        transactions::WorldTxn,
    },
};

pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub active: bool,
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
    simulation: Simulation,
}

impl PraxsmthApi {
    pub fn new(world: World, simulation: Simulation) -> Self {
        Self {
            dialog_history: Vec::new(),
            world,
            simulation,
        }
    }

    /// Parse a world from strings containing the type definitions and world definitions.
    pub fn from_strings(types: &str, world: &str) -> Result<Self> {
        let type_defs = parse_types(types).context("parsing types")?;
        let world_defs = parse_world(world).context("parsing world")?;

        let type_mapping =
            TypeMapping::from_types(type_defs).context("constructing type mapping")?;
        let mut world = World::new(type_mapping);
        let mut simulation = Simulation::new();

        let empty_bindings = Bindings::default();

        for world_def in &world_defs {
            match world_def {
                PraxsmthWorldDefinition::AgentInfo(agent_info) => {
                    world
                        .add_agent(agent_info)
                        .with_context(|| format!("adding agent {}", agent_info.name))?;
                }
                PraxsmthWorldDefinition::Declaration(declaration) => {
                    let mut transaction = world.transaction();
                    simulation
                        .process_declaration(&mut transaction, declaration, &empty_bindings)
                        .with_context(|| {
                            format!("processing declaration {:?}", declaration.sentence)
                        })?;
                    transaction.commit();
                }
            }
        }

        Ok(Self::new(world, simulation))
    }

    /// Parse and apply a single effect (e.g. `set agent.likes { amount: 5 }`) to the
    /// world on behalf of `agent_name`, committing the change. Returns any dialog the
    /// effect produced (e.g. from `say`/`broadcast`).
    pub fn process_effect(&mut self, agent_name: &str, input: &str) -> Result<Option<Dialog>> {
        let effect = parse_effect_str(input).context("parsing effect")?;
        let bindings = Bindings::default();

        let mut transaction = self.world.transaction();
        let dialog = self
            .simulation
            .process_effect(&mut transaction, agent_name, &effect, &bindings)
            .with_context(|| format!("applying effect {:?}", effect))?;
        transaction.commit();

        Ok(dialog)
    }

    /// Parse and evaluate a single expression (e.g. `a is b and not c`) against the
    /// current world state, returning the resulting constant.
    pub fn evaluate_expression(&self, input: &str) -> Result<PraxsmthConstant> {
        let expression = parse_expression_str(input).context("parsing expression")?;
        let bindings = Bindings::default();

        self.simulation
            .evaluate_expression(&self.world, &expression, &bindings)
            .with_context(|| format!("evaluating expression {:?}", expression))
    }

    /// Get the names of the available actions for an agent.
    /// The order for this is deterministic, so that the same action will always have the same index.
    pub fn get_available_actions(
        &mut self,
        agent_id: &str,
        score_depth: usize,
    ) -> Result<Vec<AvailableAction>> {
        let actions = self
            .simulation
            .get_available_actions(&self.world, agent_id)
            .with_context(|| format!("getting available action names for agent {}", agent_id))?;

        let base_score = self
            .simulation
            .evaluate_agent_goals(&self.world, agent_id)
            .with_context(|| {
                format!(
                    "evaluating goals for agent {} before applying actions",
                    agent_id
                )
            })?;

        let mut available_actions = vec![];

        for (i, action) in actions.iter().enumerate() {
            let mut transaction = self.world.transaction();
            let score = Self::score_action(
                &mut transaction,
                &mut self.simulation,
                agent_id,
                action,
                score_depth,
            )
            .with_context(|| format!("scoring action {:?} for agent {}", action, agent_id))?;
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
        simulation: &mut Simulation,
        agent_id: &str,
        action: &ActionRef,
        depth: usize,
    ) -> Result<f64> {
        if depth == 0 {
            return Ok(0.0);
        }

        let savepoint = world.savepoint();

        simulation.process_available_action(world, action)?;

        let score = if depth == 1 {
            simulation.evaluate_agent_goals(world.inner(), agent_id)?
        } else {
            let actions = simulation.get_available_actions(world.inner(), agent_id)?;

            if actions.is_empty() {
                // This tree ends here, so return the score of the current state.
                simulation.evaluate_agent_goals(world.inner(), agent_id)?
            } else {
                let mut best_score = f64::NEG_INFINITY;
                for action in actions {
                    let action_score =
                        Self::score_action(world, simulation, agent_id, &action, depth - 1)?;
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

    /// Apply an action by its index in the list of available actions for an agent.
    pub fn apply_action(&mut self, agent_name: &str, action_index: u32) -> Result<Vec<Dialog>> {
        let actions = self
            .simulation
            .get_available_actions(&self.world, agent_name)
            .with_context(|| {
                format!(
                    "getting available actions for agent {} before apply",
                    agent_name
                )
            })?;
        let action = actions.get(action_index as usize).with_context(|| {
            format!(
                "action index {} out of bounds for agent {} (have {} actions)",
                action_index,
                agent_name,
                actions.len()
            )
        })?;

        let mut transaction = self.world.transaction();
        let dialog = self
            .simulation
            .process_available_action(&mut transaction, action)
            .with_context(|| {
                format!("applying action {} for agent {}", action_index, agent_name)
            })?;
        transaction.commit();
        self.dialog_history.extend(dialog.clone());
        Ok(dialog)
    }

    /// Gets the current emotion of the agent, if any.
    pub fn get_current_emotion(&self, agent: &str) -> Result<Option<(RelationHandle, &Relation)>> {
        Ok(self
            .world
            .get_agent(agent)
            .with_context(|| format!("could not find agent {} in world", agent))?
            .emotion
            .as_ref()
            .and_then(|handle| {
                self.world
                    .get_relation(handle.clone())
                    .map(|relation| (handle.clone(), relation))
            }))
    }

    pub fn get_agent_info(&self) -> Vec<AgentInfo> {
        self.world
            .iter_agents()
            .map(|(id, agent)| AgentInfo {
                id: id.clone(),
                name: agent.name.clone(),
                active: agent.is_active,
            })
            .collect()
    }
}
