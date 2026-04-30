use crate::world::World;

impl World {
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
    pub fn apply_action(&mut self, agent_name: &str, action_index: u32) -> Result<(), String> {
        let actions = self.get_available_actions(agent_name)?;
        let action = actions
            .get(action_index as usize)
            .ok_or("invalid action index")?;

        self.process_available_action(action)
    }
}
