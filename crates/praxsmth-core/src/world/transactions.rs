use anyhow::{Context, Result};

use crate::world::{AgentToRelation, Fields, Relation, RelationHandle, World};

/// An enumeration of possible operations that can be undone in the world. Used
/// in transactions to allow rolling back changes.
pub enum UndoOp {
    // Restore a removed relation into its previous slot in `RelationStore`.
    // Maintains index and generation for consistency with lookups.
    RestoreRelation {
        index: u32,
        generation: u32,
        relation: Relation,
    },

    // Remove a relation that was added.
    RemoveAddedRelation {
        handle: RelationHandle,
    },

    // Restore a modified relation to its previous state.
    RestoreFields {
        handle: RelationHandle,
        prior: Fields,
    },

    // Restore an agents edges to their prior state. These will be modified by
    // transactions that add or remove relations.
    RestoreAgentEdges {
        agent_id: String,
        prior_edges: Vec<AgentToRelation>,
    },

    // Restore an agent's emotion to a prior value.
    RestoreAgentEmotion {
        agent_id: String,
        prior_emotion: Option<RelationHandle>,
    },
}

pub struct UndoLog {
    ops: Vec<UndoOp>,
}

impl UndoLog {
    pub fn new() -> Self {
        Self { ops: Vec::new() }
    }

    pub fn push(&mut self, op: UndoOp) {
        self.ops.push(op);
    }
}

/// Represents a transaction on the world. Changes made through this interface
/// can be rolled back if needed by calling `rollback`.
pub struct WorldTxn<'a> {
    world: &'a mut World,
    undo_log: UndoLog,
}

impl<'a> WorldTxn<'a> {
    pub fn new(world: &'a mut World) -> Self {
        Self {
            world,
            undo_log: UndoLog::new(),
        }
    }

    /// Wrapper for `World::add_trait` that logs undoable actions.
    pub fn add_trait(
        &mut self,
        agent_id: &str,
        type_id: &str,
        fields: Fields,
    ) -> Result<RelationHandle> {
        let handle = self.world.add_trait(agent_id, type_id, fields)?;
        // Undone by removing the relation we just added. The remove function
        // called by this undo op will handle removing edges from agents.
        self.undo_log.push(UndoOp::RemoveAddedRelation {
            handle: handle.clone(),
        });
        Ok(handle)
    }

    /// Wrapper for `World::add_emotion` that logs undoable actions.
    pub fn add_emotion(
        &mut self,
        agent_id: &str,
        type_id: &str,
        fields: Fields,
    ) -> Result<RelationHandle> {
        let handle = self.world.add_emotion(agent_id, type_id, fields)?;
        // Undone by removing the relation we just added. The remove function
        // called by this undo op will handle removing edges from agents.
        self.undo_log.push(UndoOp::RemoveAddedRelation {
            handle: handle.clone(),
        });
        Ok(handle)
    }

    /// Wrapper for `World::add_binary_relation` that logs undoable actions.
    pub fn add_binary_relation(
        &mut self,
        from_id: &str,
        to_id: &str,
        edge_type_id: &str,
        fields: Fields,
    ) -> Result<RelationHandle> {
        let handle = self
            .world
            .add_binary_relation(from_id, to_id, edge_type_id, fields)?;
        // Undone by removing the relation we just added. The remove function
        // called by this undo op will handle removing edges from agents.
        self.undo_log.push(UndoOp::RemoveAddedRelation {
            handle: handle.clone(),
        });
        Ok(handle)
    }

    /// Wrapper for `World::add_practice` that logs undoable actions.
    pub fn add_practice(
        &mut self,
        participant_ids: Vec<&str>,
        type_id: &str,
        fields: Fields,
    ) -> Result<RelationHandle> {
        let handle = self.world.add_practice(participant_ids, type_id, fields)?;
        // Undone by removing the relation we just added. The remove function
        // called by this undo op will handle removing edges from agents.
        self.undo_log.push(UndoOp::RemoveAddedRelation {
            handle: handle.clone(),
        });
        Ok(handle)
    }

    /// Wrapper for `World::add_relation` that logs undoable actions.
    pub fn add_relation(&mut self, rel: Relation) -> RelationHandle {
        let handle = self.world.add_relation(rel);
        // Undone by removing the relation we just added. The remove function
        // called by this undo op will handle removing edges from agents.
        self.undo_log.push(UndoOp::RemoveAddedRelation {
            handle: handle.clone(),
        });
        handle
    }

    /// Wrapper for `World::remove_relation` that logs undoable actions,
    /// including logging the edges of any agents connected to the relation
    /// being removed, so that they can be restored on rollback.
    pub fn remove_relation(&mut self, handle: RelationHandle) -> Result<()> {
        let index = handle.index;
        let generation = handle.generation;

        let prior = self
            .world
            .get_relation(handle.clone())
            .with_context(|| format!("failed to find relation with handle {:?}", handle))?
            .clone();

        prior.edges.iter().for_each(|edge| {
            if let Some(agent) = self.world.get_agent(edge.agent()) {
                // If the agent still exists, log its edges before we remove the relation.
                self.undo_log.push(UndoOp::RestoreAgentEdges {
                    agent_id: edge.agent().to_string(),
                    prior_edges: agent.edges.clone(),
                })
                // TODO: consider emotions too
            }
        });

        self.world.remove_relation(handle)?;

        self.undo_log.push(UndoOp::RestoreRelation {
            index,
            generation,
            relation: prior,
        });

        Ok(())
    }

    /// Wrapper for `World::update_relation` that logs undoable actions.
    pub fn update_relation(&mut self, handle: RelationHandle, new_fields: Fields) -> Result<()> {
        let prior = self
            .world
            .get_relation(handle.clone())
            .with_context(|| {
                format!(
                    "failed to find relation with handle {:?} for updating fields",
                    handle
                )
            })?
            .fields
            .clone();

        self.world.update_relation(handle.clone(), new_fields)?;

        self.undo_log.push(UndoOp::RestoreFields { handle, prior });

        // TODO: if it is an emotion, handle that too

        Ok(())
    }

    pub fn commit(self) {
        // No action needed, changes are already in the world.
        // Idk why this is here... for completeness or something lmfao
    }

    pub fn rollback(self) -> Result<()> {
        for op in self.undo_log.ops.into_iter().rev() {
            match op {
                UndoOp::RestoreRelation {
                    index,
                    generation,
                    relation,
                } => {
                    self.world
                        .restore_relation(index, generation, relation)
                        .with_context(|| format!("restoring relation to index {}", index))?;
                }
                UndoOp::RemoveAddedRelation { handle } => {
                    self.world.remove_relation(handle)?;
                }
                UndoOp::RestoreFields { handle, prior } => {
                    self.world.update_relation(handle, prior)?;
                }
                UndoOp::RestoreAgentEdges {
                    agent_id,
                    prior_edges,
                } => {
                    self.world
                        .get_agent_mut(&agent_id)
                        .with_context(|| format!("failed to find agent {}", agent_id))?
                        .edges = prior_edges;
                }
                UndoOp::RestoreAgentEmotion {
                    agent_id,
                    prior_emotion,
                } => {
                    self.world
                        .get_agent_mut(&agent_id)
                        .with_context(|| format!("failed to find agent {}", agent_id))?
                        .emotion = prior_emotion;
                }
            }
        }

        Ok(())
    }
}
