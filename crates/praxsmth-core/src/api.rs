use anyhow::{Context, Result};

use crate::{
    definitions::world::PraxsmthWorldDefinition,
    parser::{types::parse_types, world::parse_world},
    types::TypeMapping,
    world::{
        Bindings, Relation, RelationHandle, World,
        simulation::{Dialog, Simulation},
    },
};

pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub active: bool,
}

/// The main API for interacting with the Praxsmth world. This is the intended
/// interface for external code to use when working with the world. It combines
/// both the `World` and `Simulation` into a single struct, and provides
/// convenience methods for common operations like parsing from strings,
/// getting available actions, and applying actions.
pub struct PraxsmthApi {
    world: World,
    simulation: Simulation,
}

impl PraxsmthApi {
    pub fn new(world: World, simulation: Simulation) -> Self {
        Self { world, simulation }
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
                    simulation
                        .process_declaration(&mut world.transaction(), declaration, &empty_bindings)
                        .with_context(|| {
                            format!("processing declaration {:?}", declaration.sentence)
                        })?;
                }
            }
        }

        Ok(Self::new(world, simulation))
    }

    /// Get the names of the available actions for an agent.
    /// The order for this is deterministic, so that the same action will always have the same index.
    pub fn get_available_action_names(&self, agent_name: &str) -> Result<Vec<String>> {
        let actions = self
            .simulation
            .get_available_actions(&self.world, agent_name)
            .with_context(|| format!("getting available action names for agent {}", agent_name))?;
        Ok(actions
            .into_iter()
            .map(|action| action.display_name)
            .collect())
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

        self.simulation
            .process_available_action(&mut self.world.transaction(), action)
            .with_context(|| format!("applying action {} for agent {}", action_index, agent_name))
    }

    /// Gets the current emotion of the agent, if any.
    pub fn get_current_emotion(&self, agent: &str) -> Result<Option<(RelationHandle, &Relation)>> {
        Ok(self
            .world
            .agents
            .get(agent)
            .with_context(|| format!("could not find agent {} in world", agent))?
            .emotion
            .as_ref()
            .and_then(|handle| {
                self.world
                    .relation_store
                    .get(handle.clone())
                    .map(|relation| (handle.clone(), relation))
            }))
    }

    pub fn get_agent_info(&self) -> Vec<AgentInfo> {
        self.world
            .agents
            .iter()
            .map(|(id, agent)| AgentInfo {
                id: id.clone(),
                name: agent.name.clone(),
                active: agent.is_active,
            })
            .collect()
    }
}
