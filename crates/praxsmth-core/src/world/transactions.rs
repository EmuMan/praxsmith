use anyhow::{Context, Result};

use crate::world::{Fields, RelationHandle, World, WorldMutation};

/// Represents a transaction on the world. Changes made through this interface
/// can be rolled back if needed by calling `rollback`.
pub struct WorldTxn<'a> {
    world: &'a mut World,
    mutation_log: Vec<WorldMutation>,
}

impl<'a> WorldTxn<'a> {
    pub fn new(world: &'a mut World) -> Self {
        Self {
            world,
            mutation_log: Vec::new(),
        }
    }

    /// Public read-only access to the world for querying.
    pub fn inner(&self) -> &World {
        self.world
    }

    /// Wrapper for `World::add_trait` that logs undoable actions.
    pub fn add_trait(
        &mut self,
        agent_id: &str,
        type_id: &str,
        fields: Fields,
    ) -> Result<RelationHandle> {
        let created = self.world.add_trait(agent_id, type_id, fields)?;
        self.mutation_log.extend(created.mutations);
        Ok(created.handle)
    }

    /// Wrapper for `World::add_emotion` that logs undoable actions.
    pub fn add_emotion(
        &mut self,
        agent_id: &str,
        type_id: &str,
        fields: Fields,
    ) -> Result<RelationHandle> {
        let created = self.world.add_emotion(agent_id, type_id, fields)?;
        self.mutation_log.extend(created.mutations);
        Ok(created.handle)
    }

    /// Wrapper for `World::add_binary_relation` that logs undoable actions.
    pub fn add_binary_relation(
        &mut self,
        from_id: &str,
        to_id: &str,
        edge_type_id: &str,
        fields: Fields,
    ) -> Result<RelationHandle> {
        let created = self
            .world
            .add_binary_relation(from_id, to_id, edge_type_id, fields)?;
        self.mutation_log.extend(created.mutations);
        Ok(created.handle)
    }

    /// Wrapper for `World::add_practice` that logs undoable actions.
    pub fn add_practice(
        &mut self,
        participant_ids: Vec<&str>,
        type_id: &str,
        fields: Fields,
    ) -> Result<RelationHandle> {
        let created = self.world.add_practice(participant_ids, type_id, fields)?;
        self.mutation_log.extend(created.mutations);
        Ok(created.handle)
    }

    /// Wrapper for `World::remove_relation` that logs undoable actions,
    /// including logging the edges of any agents connected to the relation
    /// being removed, so that they can be restored on rollback.
    pub fn remove_relation(&mut self, handle: RelationHandle) -> Result<()> {
        self.mutation_log
            .extend(self.world.remove_relation(handle)?);
        Ok(())
    }

    /// Wrapper for `World::update_relation` that logs undoable actions.
    pub fn update_relation(&mut self, handle: RelationHandle, new_fields: Fields) -> Result<()> {
        self.mutation_log
            .push(self.world.update_relation(handle, new_fields)?);
        Ok(())
    }

    /// Wrapper for setting an agent's active state that logs undoable actions.
    pub fn set_agent_active(&mut self, name: &str, active: bool) -> Result<()> {
        self.mutation_log
            .push(self.world.set_agent_active(name, active)?);
        Ok(())
    }

    pub fn rollback(self) -> Result<()> {
        log::info!(
            "rolling back transaction with {} undo operations",
            self.mutation_log.len()
        );
        for mutation in self.mutation_log.into_iter().rev() {
            let mutation_str = format!("{:?}", mutation);
            self.world
                .undo_mutation(mutation)
                .with_context(|| format!("undoing mutation {}", mutation_str))?;
        }

        Ok(())
    }
}

impl World {
    /// Start a new transaction on the world. Changes made through the returned
    /// `WorldTxn` can be rolled back if needed by calling `rollback`.
    pub fn transaction(&mut self) -> WorldTxn<'_> {
        WorldTxn::new(self)
    }
}
