use crate::{
    definitions::world::PraxsmthWorldDefinition,
    parser::{types::parse_types, world::parse_world},
    types::TypeMapping,
    world::{Relation, RelationHandle, World, simulation::Dialog},
};

impl World {
    /// Parse a world from strings containing the type definitions and world definitions.
    pub fn from_strings(types: &str, world: &str) -> Result<Self, String> {
        let type_defs = parse_types(types).map_err(|e| format!("failed to parse types: {}", e))?;
        let world_defs = parse_world(world).map_err(|e| format!("failed to parse world: {}", e))?;

        let type_mapping = TypeMapping::from_types(type_defs)?;
        let mut world = World::new(type_mapping);

        for world_def in &world_defs {
            match world_def {
                PraxsmthWorldDefinition::AgentInfo(agent_info) => {
                    world.add_agent(agent_info)?;
                }
                PraxsmthWorldDefinition::Declaration(declaration) => {
                    world.process_declaration(declaration)?;
                }
            }
        }

        Ok(world)
    }

    /// Get the names of the available actions for an agent.
    /// The order for this is deterministic, so that the same action will always have the same index.
    pub fn get_available_action_names(&self, agent_name: &str) -> Result<Vec<String>, String> {
        let actions = self.get_available_actions(agent_name)?;
        Ok(actions
            .into_iter()
            .map(|action| action.display_name)
            .collect())
    }

    /// Apply an action by its index in the list of available actions for an agent.
    /// I unfortunately don't think there's an easy better way to do this.
    pub fn apply_action(
        &mut self,
        agent_name: &str,
        action_index: u32,
    ) -> Result<Vec<Dialog>, String> {
        let actions = self.get_available_actions(agent_name)?;
        let action = actions
            .get(action_index as usize)
            .ok_or("invalid action index")?;

        self.process_available_action(action)
    }

    /// Gets the current emotion of the agent, if any.
    pub fn get_current_emotion(
        &self,
        agent: &str,
    ) -> Result<Option<(RelationHandle, &Relation)>, String> {
        Ok(self
            .agents
            .get(agent)
            .ok_or_else(|| format!("could not find agent {} in world", agent))?
            .emotion
            .as_ref()
            .and_then(|handle| {
                self.relation_store
                    .get(handle.clone())
                    .map(|relation| (handle.clone(), relation))
            }))
    }
}
