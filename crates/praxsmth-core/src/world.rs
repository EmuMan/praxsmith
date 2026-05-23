use std::{collections::HashMap, fmt};

use anyhow::{Context, Result, bail};

use crate::{
    types::{FieldType, FieldTypes, RelationType, RelationTypeData, RelationTypeMap},
    values::Constant,
    world::{bindings::Bindings, goals::Goal},
};

pub mod bindings;
pub mod goals;
pub mod simulation;
pub mod transactions;

type Fields = HashMap<String, Constant>;

// TODO: verify this works correctly in all cases, and add more detailed error messages
fn verify_fields(fields: &Fields, field_types: &FieldTypes, require_all: bool) -> Result<()> {
    if require_all {
        for field_name in field_types.iter_names() {
            if !fields.contains_key(field_name) {
                bail!("field {} is required but not present", field_name);
            }
        }
    }
    for (field_name, field_value) in fields {
        match field_types.get(field_name) {
            Some(expected_type) => match (expected_type, field_value) {
                (FieldType::NumberRange(start, end), Constant::Number(n)) => {
                    if n < start || n > end {
                        bail!(
                            "field {} value {} is out of range {}..{}",
                            field_name,
                            n,
                            start,
                            end
                        );
                    }
                }
                (FieldType::VariantList(variants), Constant::Variant(v)) => {
                    if !variants.contains(v) {
                        bail!(
                            "field {} value {} is not in variant list {:?}",
                            field_name,
                            v,
                            variants
                        );
                    }
                }
                _ => {
                    bail!(
                        "field {} has type mismatch: expected {}, got {}",
                        field_name,
                        expected_type,
                        field_value
                    );
                }
            },
            None => {
                bail!("field {} is not defined in type", field_name);
            }
        }
    }
    Ok(())
}

/// Represents an edge from an agent to a relation.
///
/// Fields:
/// - `index`: The index of the agent in the relation's edges list. This is
///   used primarily for directional edges, where the position of the agent in
///   the edge list determines its role (e.g. forward vs backward).
/// - `relation_type`: The type of the relation this edge points to. This is
///   used for quick access to the relation's type without needing to look it
///   up in the relation store.
/// - `relation_handle`: A handle to the relation this edge points to.
#[derive(Debug, Clone)]
pub struct AgentToRelation {
    pub index: usize,
    pub relation_type: String,
    pub relation_handle: RelationHandle,
}

#[derive(Debug, Clone)]
pub enum RelationToAgent {
    Solo(String),
    Forward(String),
    Backward(String),
    Unordered(String),
    Ordered(String),
}

impl RelationToAgent {
    pub fn agent(&self) -> &str {
        match self {
            RelationToAgent::Solo(a)
            | RelationToAgent::Forward(a)
            | RelationToAgent::Backward(a)
            | RelationToAgent::Unordered(a)
            | RelationToAgent::Ordered(a) => &a,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Relation {
    pub type_name: String,
    edges: Vec<RelationToAgent>,
    pub fields: Fields,
    pub data: RelationData,
}

impl Relation {
    pub fn update_fields(&mut self, new_fields: Fields, field_defs: &FieldTypes) -> Result<()> {
        verify_fields(&new_fields, &field_defs, false)
            .context("verifying new fields against existing type definition")?;
        for (field_name, field_value) in new_fields {
            self.fields.insert(field_name, field_value);
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum RelationData {
    Trait {
        agent: String,
    },
    Emotion {
        agent: String,
    },
    Directional {
        agent_from: String,
        agent_to: String,
    },
    Reciprocal {
        agent_1: String,
        agent_2: String,
    },
    Evaluation {
        agent_from: String,
        agent_to: String,
        reason: String,
    },
    Practice {
        agents: Vec<String>,
        bindings: Bindings,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RelationHandle {
    index: u32,
    generation: u32,
}

struct RelationStoreSlot {
    value: Option<Relation>,
    generation: u32,
}

pub struct RelationStore {
    slots: Vec<RelationStoreSlot>,
    open_indices: Vec<usize>,
}

impl RelationStore {
    pub fn new() -> Self {
        RelationStore {
            slots: Vec::new(),
            open_indices: Vec::new(),
        }
    }

    pub fn peek_next_two_handles(&self) -> (RelationHandle, RelationHandle) {
        if self.open_indices.is_empty() {
            let new_index = self.slots.len();
            (
                RelationHandle {
                    index: new_index as u32,
                    generation: 0,
                },
                RelationHandle {
                    index: (new_index + 1) as u32,
                    generation: 0,
                },
            )
        } else if self.open_indices.len() == 1 {
            let slot_index = self.open_indices[0];
            (
                RelationHandle {
                    index: slot_index as u32,
                    generation: self.slots[slot_index].generation,
                },
                RelationHandle {
                    index: self.slots.len() as u32,
                    generation: 0,
                },
            )
        } else {
            let slot_index1 = self.open_indices[self.open_indices.len() - 1];
            let slot_index2 = self.open_indices[self.open_indices.len() - 2];
            (
                RelationHandle {
                    index: slot_index1 as u32,
                    generation: self.slots[slot_index1].generation,
                },
                RelationHandle {
                    index: slot_index2 as u32,
                    generation: self.slots[slot_index2].generation,
                },
            )
        }
    }

    pub fn add(&mut self, relation: Relation) -> RelationHandle {
        if let Some(slot_index) = self.open_indices.pop() {
            let slot = &mut self.slots[slot_index];
            slot.value = Some(relation);
            RelationHandle {
                index: slot_index as u32,
                generation: slot.generation,
            }
        } else {
            let new_index = self.slots.len();
            self.slots.push(RelationStoreSlot {
                value: Some(relation),
                generation: 0,
            });
            RelationHandle {
                index: new_index as u32,
                generation: 0,
            }
        }
    }

    /// Restores a relation into the store at the given index and generation.
    /// This is used for undoing a removal, and to preserve handle consistency,
    /// it bypasses the normal checks and advancements the store would normally
    /// do. Do NOT use this for anything other than undoing a removal, as it
    /// can easily lead to inconsistent state if used incorrectly.
    ///
    /// Returns an error if there is a value in the slot already.
    pub fn restore(&mut self, handle: RelationHandle, relation: Relation) -> Result<()> {
        if handle.index as usize >= self.slots.len() {
            bail!(
                "cannot restore relation at index {}: index out of bounds",
                handle.index
            );
        }
        let slot = &mut self.slots[handle.index as usize];
        if slot.value.is_some() {
            bail!(
                "cannot restore relation at index {}: slot is not empty",
                handle.index
            );
        }
        slot.value = Some(relation);
        slot.generation = handle.generation;
        // Remove this index from open_indices if it's there, since the slot is now occupied again
        if let Some(pos) = self
            .open_indices
            .iter()
            .position(|&i| i == handle.index as usize)
        {
            self.open_indices.remove(pos);
        }
        Ok(())
    }

    pub fn get(&self, handle: RelationHandle) -> Option<&Relation> {
        self.slots.get(handle.index as usize).and_then(|slot| {
            if slot.generation == handle.generation {
                slot.value.as_ref()
            } else {
                None
            }
        })
    }

    pub fn get_mut(&mut self, handle: RelationHandle) -> Option<&mut Relation> {
        self.slots.get_mut(handle.index as usize).and_then(|slot| {
            if slot.generation == handle.generation {
                slot.value.as_mut()
            } else {
                None
            }
        })
    }

    pub fn remove(&mut self, handle: RelationHandle) -> Result<()> {
        if let Some(slot) = self.slots.get_mut(handle.index as usize) {
            if slot.generation == handle.generation {
                slot.value = None;
                slot.generation += 1;
                self.open_indices.push(handle.index as usize);
                Ok(())
            } else {
                bail!(
                    "invalid handle generation (handle gen {}, slot gen {})",
                    handle.generation,
                    slot.generation
                );
            }
        } else {
            bail!("invalid handle index {}", handle.index);
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentInitInfo {
    pub id: String,
    pub name: String,
    pub active: bool,
    pub goals: Vec<Goal>,
}

impl fmt::Display for AgentInitInfo {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.goals.is_empty() {
            write!(f, "{}", self.name)
        } else {
            let goals_str: Vec<_> = self
                .goals
                .iter()
                .map(|g| format!("goal({}): {:?}", g.weight, g.expression))
                .collect();
            write!(f, "{} {{{}}}", self.name, goals_str.join(", "))
        }
    }
}

pub struct Agent {
    pub name: String,
    pub edges: Vec<AgentToRelation>,
    // Quick access field for the singular emotion they might have
    pub emotion: Option<RelationHandle>,
    pub is_active: bool,
    pub goals: Vec<Goal>,
}

impl Agent {
    pub fn new(name: String, is_active: bool, goals: Vec<Goal>) -> Self {
        Agent {
            name,
            edges: Vec::new(),
            emotion: None,
            is_active,
            goals,
        }
    }

    pub fn remove_edges_to(&mut self, handle: RelationHandle) {
        self.edges.retain(|edge| edge.relation_handle != handle);
    }
}

#[derive(Debug, Clone)]
pub struct RelationCreated {
    pub handle: RelationHandle,
    pub mutations: Vec<WorldMutation>,
}

#[derive(Debug, Clone)]
pub enum WorldMutation {
    RelationAdded {
        handle: RelationHandle,
    },
    RelationRemoved {
        handle: RelationHandle,
        prior: Relation,
    },
    RelationFieldsUpdated {
        handle: RelationHandle,
        prior_fields: Fields,
    },
    AgentSetActive {
        agent_id: String,
        prior_active: bool,
    },
    AgentEdgesUpdated {
        agent_id: String,
        prior_edges: Vec<AgentToRelation>,
    },
    AgentEmotionUpdated {
        agent_id: String,
        prior_emotion: Option<RelationHandle>,
    },
}

pub struct World {
    agents: HashMap<String, Agent>,
    relation_type_map: RelationTypeMap,
    relation_store: RelationStore,
}

impl World {
    pub fn new(type_map: RelationTypeMap) -> Self {
        World {
            agents: HashMap::new(),
            relation_type_map: type_map,
            relation_store: RelationStore::new(),
        }
    }

    pub fn get_relation_type_map(&self) -> &RelationTypeMap {
        &self.relation_type_map
    }

    pub fn iter_relations(&self) -> impl Iterator<Item = (RelationHandle, &Relation)> {
        self.relation_store
            .slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                slot.value.as_ref().map(|rel| {
                    (
                        RelationHandle {
                            index: index as u32,
                            generation: slot.generation,
                        },
                        rel,
                    )
                })
            })
    }

    pub fn iter_agent_relations<'a, 'b>(
        &'a self,
        agent: &'b Agent,
    ) -> impl Iterator<Item = (&'b AgentToRelation, &'a Relation)> {
        agent.edges.iter().filter_map(|edge| {
            let handle = edge.relation_handle.clone();
            self.relation_store
                .get(handle.clone())
                .map(|rel| (edge, rel))
        })
    }

    fn format_string(&self, string: &str, bindings: &Bindings) -> Result<String> {
        let mut result = string.to_string();
        for (var, value) in bindings {
            let agent = self
                .get_agent(value)
                .with_context(|| format!("looking up agent {} for string formatting", value))?;
            let placeholder = format!("[{}]", var);
            result = result.replace(&placeholder, &agent.name);
        }
        Ok(result)
    }

    fn resolve_binding_or_same(string: &str, bindings: &Bindings) -> String {
        bindings
            .get(string)
            .cloned()
            .unwrap_or_else(|| string.to_string())
    }

    pub fn add_agent(&mut self, info: &AgentInitInfo) -> Result<()> {
        if self.agents.contains_key(&info.id) {
            bail!("agent with id {} already exists", &info.id);
        }
        self.agents.insert(
            info.id.clone(),
            Agent::new(info.name.clone(), info.active, info.goals.clone()),
        );
        Ok(())
    }

    pub fn get_agent(&self, name: &str) -> Option<&Agent> {
        self.agents.get(name)
    }

    pub fn get_agent_mut(&mut self, name: &str) -> Option<&mut Agent> {
        self.agents.get_mut(name)
    }

    pub fn iter_agents(&self) -> impl Iterator<Item = (&String, &Agent)> {
        self.agents.iter()
    }

    pub fn set_agent_active(&mut self, name: &str, active: bool) -> Result<WorldMutation> {
        let agent = self
            .agents
            .get_mut(name)
            .with_context(|| format!("looking up agent {} for activation", name))?;
        let prior_active = agent.is_active;
        agent.is_active = active;
        Ok(WorldMutation::AgentSetActive {
            agent_id: name.to_string(),
            prior_active,
        })
    }

    pub fn get_relation(&self, handle: RelationHandle) -> Option<&Relation> {
        self.relation_store.get(handle)
    }

    fn add_relation(&mut self, relation: Relation) -> RelationCreated {
        let handle = self.relation_store.add(relation);
        RelationCreated {
            handle: handle.clone(),
            mutations: vec![WorldMutation::RelationAdded { handle }],
        }
    }

    pub fn update_relation(
        &mut self,
        handle: RelationHandle,
        new_fields: Fields,
    ) -> Result<WorldMutation> {
        let relation = self
            .relation_store
            .get_mut(handle.clone())
            .with_context(|| format!("looking up relation {:?} for update", handle))?;
        let relation_type = self
            .relation_type_map
            .get_type(&relation.type_name)
            .with_context(|| {
                format!(
                    "looking up type {} for relation {:?}",
                    relation.type_name, handle
                )
            })?;

        let prior_fields = relation.fields.clone();

        match &mut relation.data {
            RelationData::Trait { .. }
            | RelationData::Emotion { .. }
            | RelationData::Directional { .. }
            | RelationData::Reciprocal { .. }
            | RelationData::Practice { .. } => relation
                .update_fields(new_fields, &relation_type.fields)
                .with_context(|| format!("updating fields on relation {:?}", handle))?,
            RelationData::Evaluation { reason, .. } => {
                if let Some(new_reason) = new_fields.get("reason") {
                    let Constant::String(reason_str) = new_reason else {
                        bail!("evaluation edge 'reason' field must be a string");
                    };
                    *reason = reason_str.clone();
                }
                relation
                    .update_fields(new_fields, &relation_type.fields)
                    .with_context(|| {
                        format!("updating fields on evaluation relation {:?}", handle)
                    })?
            }
        }

        Ok(WorldMutation::RelationFieldsUpdated {
            handle,
            prior_fields,
        })
    }

    pub fn remove_relation(&mut self, handle: RelationHandle) -> Result<Vec<WorldMutation>> {
        let mut mutations = Vec::new();

        let relation = self
            .relation_store
            .get(handle.clone())
            .with_context(|| format!("looking up relation {:?} for removal", handle))?;
        relation.edges.iter().for_each(|edge_to_agent| {
            let agent_name = edge_to_agent.agent();
            if let Some(agent) = self.agents.get_mut(agent_name) {
                mutations.push(WorldMutation::AgentEdgesUpdated {
                    agent_id: agent_name.to_string(),
                    prior_edges: agent.edges.clone(),
                });
                agent.remove_edges_to(handle.clone());
            } else {
                panic!(
                    "agent with name {} not found when removing relation with handle {:?}",
                    agent_name, handle
                );
            }
        });

        mutations.push(WorldMutation::RelationRemoved {
            handle: handle.clone(),
            prior: relation.clone(),
        });

        self.relation_store
            .remove(handle.clone())
            .with_context(|| format!("removing relation {:?} from store", handle))?;

        Ok(mutations)
    }

    fn validate_agent(&self, name: &str) -> Result<()> {
        if self.agents.contains_key(name) {
            Ok(())
        } else {
            bail!("agent with name {} not found", name);
        }
    }

    fn validate_agents(&self, names: &[&str]) -> Result<()> {
        for name in names {
            self.validate_agent(name)
                .with_context(|| format!("validating agent {:?}", name))?;
        }
        Ok(())
    }

    fn validate_type_fields(&self, type_name: &str, fields: &Fields) -> Result<()> {
        let edge_type = self
            .relation_type_map
            .get_type(type_name)
            .with_context(|| format!("looking up type {} in type mapping", type_name))?;
        verify_fields(fields, &edge_type.fields, true)
            .with_context(|| format!("verifying fields against type {}", type_name))
    }

    fn expect_type<'a>(
        &'a self,
        type_id: &str,
        label: &str,
        check: impl Fn(&RelationTypeData) -> bool,
    ) -> Result<&'a RelationType> {
        let t = self
            .relation_type_map
            .get_type(type_id)
            .with_context(|| format!("type {} not found in type mapping", type_id))?;
        if !check(&t.data) {
            bail!("type {} is not a {} type", type_id, label);
        }
        Ok(t)
    }

    /// Adds a trait to the world.
    ///
    /// A trait is a relation that connects a single agent to a type, and
    /// represents a property or characteristic of that agent. As there is only
    /// ever one edge to the relation, that edge will always have index `0`.
    ///
    /// Returns an error if the type does not exist or is not a trait type,
    /// if the agent does not exist, or if the fields do not match the type
    /// definition. Otherwise, returns an associated `RelationCreated`
    /// containing a handle to the newly created relation and a list of
    /// mutations that were applied to the world as part of creating the
    /// relation.
    pub fn add_trait(
        &mut self,
        agent_id: &str,
        type_id: &str,
        fields: Fields,
    ) -> Result<RelationCreated> {
        let trait_ctx = || format!("adding trait {} to agent {}", type_id, agent_id);

        self.expect_type(type_id, "trait", |d| matches!(d, RelationTypeData::Trait))
            .with_context(trait_ctx)?;

        self.validate_type_fields(type_id, &fields)
            .with_context(trait_ctx)?;

        self.validate_agent(agent_id).with_context(trait_ctx)?;

        let mut created = self.add_relation(Relation {
            type_name: type_id.to_string(),
            edges: vec![RelationToAgent::Solo(agent_id.to_string())],
            fields,
            data: RelationData::Trait {
                agent: agent_id.to_string(),
            },
        });

        let agent = self.agents.get_mut(agent_id).unwrap();

        created.mutations.push(WorldMutation::AgentEdgesUpdated {
            agent_id: agent_id.to_string(),
            prior_edges: agent.edges.clone(),
        });

        agent.edges.push(AgentToRelation {
            index: 0,
            relation_type: type_id.to_string(),
            relation_handle: created.handle.clone(),
        });

        Ok(created)
    }

    /// Retrieves a trait relation for the given agent and type, if it exists.
    ///
    /// Returns an error if the type does not exist or is not a trait type, or
    /// if the agent does not exist. Otherwise, returns `Ok(None)` if the agent
    /// does not have that relation, and `Ok(Some((edge, relation)))` if they
    /// do.
    pub fn get_trait(
        &self,
        agent_id: &str,
        type_id: &str,
    ) -> Result<Option<(&AgentToRelation, &Relation)>> {
        let trait_ctx = || format!("getting trait {} from agent {}", type_id, agent_id);

        self.expect_type(type_id, "trait", |d| matches!(d, RelationTypeData::Trait))
            .with_context(trait_ctx)?;

        self.validate_agent(agent_id).with_context(trait_ctx)?;

        let agent = self.agents.get(agent_id).unwrap();

        Ok(agent.edges.iter().find_map(|edge| {
            if edge.relation_type == type_id {
                if let Some(relation) = self.relation_store.get(edge.relation_handle.clone()) {
                    return Some((edge, relation));
                }
            }
            None
        }))
    }

    /// Adds an emotion to the world.
    ///
    /// An emotion is a relation that connects a single agent to a type, and
    /// represents the sole, short-term emotion that the agent may have. As
    /// there is only ever one edge to the relation, that edge will always have
    /// index `0`.
    ///
    /// Returns an error if the type does not exist or is not an emotion type,
    /// if the agent does not exist, or if the fields do not match the type
    /// definition. Otherwise, returns an associated `RelationCreated`
    /// containing a handle to the newly created relation and a list of
    /// mutations that were applied to the world as part of creating the
    /// relation.
    pub fn add_emotion(
        &mut self,
        agent_id: &str,
        type_id: &str,
        fields: Fields,
    ) -> Result<RelationCreated> {
        let emotion_ctx = || format!("adding emotion {} to agent {}", type_id, agent_id);

        self.expect_type(type_id, "emotion", |d| {
            matches!(d, RelationTypeData::Emotion)
        })
        .with_context(emotion_ctx)?;

        self.validate_type_fields(type_id, &fields)
            .with_context(emotion_ctx)?;

        self.validate_agent(agent_id).with_context(emotion_ctx)?;

        let mut created = self.add_relation(Relation {
            type_name: type_id.to_string(),
            edges: vec![RelationToAgent::Solo(agent_id.to_string())],
            fields,
            data: RelationData::Emotion {
                agent: agent_id.to_string(),
            },
        });

        let agent = self.agents.get_mut(agent_id).unwrap();

        let old_emotion_handle = agent.emotion.clone();

        created.mutations.push(WorldMutation::AgentEdgesUpdated {
            agent_id: agent_id.to_string(),
            prior_edges: agent.edges.clone(),
        });
        created.mutations.push(WorldMutation::AgentEmotionUpdated {
            agent_id: agent_id.to_string(),
            prior_emotion: old_emotion_handle.clone(),
        });

        agent.edges.push(AgentToRelation {
            index: 0,
            relation_type: type_id.to_string(),
            relation_handle: created.handle.clone(),
        });
        agent.emotion = Some(created.handle.clone());

        // Remove the old emotion edge for this agent, since an agent can only have one emotion edge at a time
        if let Some(old_emotion_handle) = old_emotion_handle {
            let old_removal_mutations = self
                .remove_relation(old_emotion_handle)
                .with_context(|| format!("replacing prior emotion edge on agent {}", agent_id))?;
            created.mutations.extend(old_removal_mutations);
        }

        Ok(created)
    }

    /// Retrieves an emotion relation for the given agent and type, if it
    /// exists.
    ///
    /// Returns an error if the type does not exist or is not an emotion type,
    /// or if the agent does not exist. Otherwise, returns `Ok(None)` if the
    /// agent does not have that relation, and `Ok(Some((edge, relation)))` if
    /// they do.
    pub fn get_emotion(
        &self,
        agent_id: &str,
        type_id: &str,
    ) -> Result<Option<(&AgentToRelation, &Relation)>> {
        let emotion_ctx = || format!("getting emotion {} from agent {}", type_id, agent_id);

        self.expect_type(type_id, "emotion", |d| {
            matches!(d, RelationTypeData::Emotion)
        })
        .with_context(emotion_ctx)?;

        self.validate_agent(agent_id).with_context(emotion_ctx)?;

        let agent = self.agents.get(agent_id).unwrap();

        Ok(agent.edges.iter().find_map(|edge| {
            if edge.relation_type == type_id {
                if let Some(relation) = self.relation_store.get(edge.relation_handle.clone()) {
                    return Some((edge, relation));
                }
            }
            None
        }))
    }

    /// Adds a directional relation to the world.
    ///
    /// A directional relationship connects two agents in a directed way, with
    /// a distinct "from" agent and "to" agent. The from agent is always at
    /// index `0` in the relation's edges list, and the to agent is always at
    /// index `1`.
    ///
    /// Returns an error if the type does not exist or is not a directional
    /// type, if either agent does not exist, or if the fields do not match the
    /// type definition. Otherwise, returns an associated `RelationCreated`
    /// containing a handle to the newly created relation and a list of
    /// mutations that were applied to the world as part of creating the
    /// relation.
    pub fn add_directional(
        &mut self,
        from_id: &str,
        to_id: &str,
        type_id: &str,
        fields: Fields,
    ) -> Result<RelationCreated> {
        let directional_ctx = || {
            format!(
                "adding directional {} from {} to {}",
                type_id, from_id, to_id
            )
        };

        self.expect_type(type_id, "directional", |d| {
            matches!(d, RelationTypeData::Directional { .. })
        })
        .with_context(directional_ctx)?;

        self.validate_type_fields(type_id, &fields)
            .with_context(directional_ctx)?;

        self.validate_agents(&[from_id, to_id])
            .with_context(directional_ctx)?;

        let exclusive = matches!(
            self.relation_type_map.get_type(type_id),
            Some(t) if matches!(&t.data, RelationTypeData::Directional { exclusive: true, .. })
        );

        let existing = if exclusive {
            self.agents
                .get(from_id)
                .into_iter()
                .flat_map(|a| a.edges.iter())
                .find_map(|edge| {
                    // From agents are always index zero, so if this agent has
                    // an index zero edge going to this type then it's the one
                    // we want to replace
                    if edge.relation_type == type_id && edge.index == 0 {
                        return Some(edge.relation_handle.clone());
                    }
                    None
                })
        } else {
            None
        };

        let mut created = self.add_relation(Relation {
            type_name: type_id.to_string(),
            edges: vec![
                RelationToAgent::Forward(from_id.to_string()),
                RelationToAgent::Backward(to_id.to_string()),
            ],
            fields,
            data: RelationData::Directional {
                agent_from: from_id.to_string(),
                agent_to: to_id.to_string(),
            },
        });

        if let Some(existing) = existing {
            let removal_mutations = self
                .remove_relation(existing)
                .with_context(|| format!("removing existing exclusive directional relation from {} for new relation from {} to {}", from_id, from_id, to_id))?;
            created.mutations.extend(removal_mutations);
        }

        let from_agent = self.agents.get_mut(from_id).unwrap();

        created.mutations.push(WorldMutation::AgentEdgesUpdated {
            agent_id: from_id.to_string(),
            prior_edges: from_agent.edges.clone(),
        });

        from_agent.edges.push(AgentToRelation {
            index: 0,
            relation_type: type_id.to_string(),
            relation_handle: created.handle.clone(),
        });

        let to_agent = self.agents.get_mut(to_id).unwrap();

        created.mutations.push(WorldMutation::AgentEdgesUpdated {
            agent_id: to_id.to_string(),
            prior_edges: to_agent.edges.clone(),
        });

        to_agent.edges.push(AgentToRelation {
            index: 1,
            relation_type: type_id.to_string(),
            relation_handle: created.handle.clone(),
        });

        Ok(created)
    }

    /// Retrieves a directional relation for the given from and to agents and
    /// type, if it exists. Works on both primary and complement type names,
    /// but requires correct ordering of arguments.
    ///
    /// Due to the nature of directional relationships, this lookup only works
    /// if the `from` and `to` parameters are ordered correctly. For example,
    /// if `x.is_in.y` exists, then `get_directional(x, y, is_in)` will find
    /// it, but `get_directional(y, x, is_in)` will not. However, in this case,
    /// the complement of this relation (e.g. `y.contains.x`) will be properly
    /// handled, as the complement is resolved to the primary type and the from
    /// and to are reversed in the lookup.
    ///
    /// Returns an error if the type does not exist or is not a directional
    /// type, or if either of the agents do not exist. Otherwise, returns
    /// `Ok(None)` if the agents do not have that relation, and
    /// `Ok(Some((edge, relation)))` if they do.
    pub fn get_directional(
        &self,
        from_id: &str,
        to_id: &str,
        type_id: &str,
    ) -> Result<Option<(&AgentToRelation, &Relation)>> {
        // If the edge is a complement, perform a search on the primary type
        // with the from and to reversed. This allows us to find the edge
        // regardless of which direction it's referred to as.
        if let Some(primary_type_name) = self.relation_type_map.get_complement_entry(type_id) {
            return self.get_directional(to_id, from_id, primary_type_name);
        }

        // Error checking!
        let directional_ctx = || {
            format!(
                "getting directional {} from {} to {}",
                type_id, from_id, to_id
            )
        };

        self.expect_type(type_id, "directional", |d| {
            matches!(d, RelationTypeData::Directional { .. })
        })
        .with_context(directional_ctx)?;

        self.validate_agents(&[from_id, to_id])
            .with_context(directional_ctx)?;

        let from_agent = self.agents.get(from_id).unwrap();

        Ok(from_agent.edges.iter().find_map(|edge| {
            if edge.relation_type == type_id && edge.index == 0 {
                if let Some(relation) = self.relation_store.get(edge.relation_handle.clone()) {
                    if let RelationData::Directional { agent_to, .. } = &relation.data {
                        if agent_to == to_id {
                            return Some((edge, relation));
                        }
                    }
                }
            }
            None
        }))
    }

    /// Adds a reciprocal relation to the world.
    ///
    /// A reciprocal relationship connects two agents in a non-directed way,
    /// with no order distinction between the two agents. The two agents are
    /// always at index `0` and `1` in the relation's edges list, but there is
    /// no significance to this ordering and as with the associated lookup,
    /// the relation can be found regardless of the order of the agents.
    ///
    /// Returns an error if the type does not exist or is not a reciprocal
    /// type, if either agent does not exist, or if the fields do not match the
    /// type definition. Otherwise, returns an associated `RelationCreated`
    /// containing a handle to the newly created relation and a list of
    /// mutations that were applied to the world as part of creating the
    /// relation.
    pub fn add_reciprocal(
        &mut self,
        agent_1_id: &str,
        agent_2_id: &str,
        type_id: &str,
        fields: Fields,
    ) -> Result<RelationCreated> {
        let reciprocal_ctx = || {
            format!(
                "adding reciprocal {} between {} and {}",
                type_id, agent_1_id, agent_2_id
            )
        };

        self.expect_type(type_id, "reciprocal", |d| {
            matches!(d, RelationTypeData::Reciprocal { .. })
        })
        .with_context(reciprocal_ctx)?;

        self.validate_type_fields(type_id, &fields)
            .with_context(reciprocal_ctx)?;

        self.validate_agents(&[agent_1_id, agent_2_id])
            .with_context(reciprocal_ctx)?;

        let mut created = self.add_relation(Relation {
            type_name: type_id.to_string(),
            edges: vec![
                RelationToAgent::Unordered(agent_1_id.to_string()),
                RelationToAgent::Unordered(agent_2_id.to_string()),
            ],
            fields,
            data: RelationData::Reciprocal {
                agent_1: agent_1_id.to_string(),
                agent_2: agent_2_id.to_string(),
            },
        });

        let agent_1 = self.agents.get_mut(agent_1_id).unwrap();

        created.mutations.push(WorldMutation::AgentEdgesUpdated {
            agent_id: agent_1_id.to_string(),
            prior_edges: agent_1.edges.clone(),
        });

        agent_1.edges.push(AgentToRelation {
            index: 0,
            relation_type: type_id.to_string(),
            relation_handle: created.handle.clone(),
        });

        let agent_2 = self.agents.get_mut(agent_2_id).unwrap();

        created.mutations.push(WorldMutation::AgentEdgesUpdated {
            agent_id: agent_2_id.to_string(),
            prior_edges: agent_2.edges.clone(),
        });

        agent_2.edges.push(AgentToRelation {
            index: 1,
            relation_type: type_id.to_string(),
            relation_handle: created.handle.clone(),
        });

        Ok(created)
    }

    /// Retrieves a reciprocal relation for the given two agents and type, if
    /// it exists.
    ///
    /// Since reciprocal relations have no order distinction between the two
    /// agents, this lookup will find the relation regardless of the order of
    /// `agent_1` and `agent_2`. For example, if `x.is_friends_with.y` exists
    /// as a reciprocal relation, then both
    /// `get_reciprocal(x, y, is_friends_with)` and
    /// `get_reciprocal(y, x, is_friends_with)` will find it.
    ///
    /// Returns an error if the type does not exist or is not a reciprocal
    /// type, or if either of the agents do not exist. Otherwise, returns
    /// `Ok(None)` if the agents do not have that relation, and
    /// `Ok(Some((edge, relation)))` if they do.
    pub fn get_reciprocal(
        &self,
        agent_1_id: &str,
        agent_2_id: &str,
        type_id: &str,
    ) -> Result<Option<(&AgentToRelation, &Relation)>> {
        // Error checking!
        let reciprocal_ctx = || {
            format!(
                "getting reciprocal {} between {} and {}",
                type_id, agent_1_id, agent_2_id
            )
        };

        self.expect_type(type_id, "reciprocal", |d| {
            matches!(d, RelationTypeData::Reciprocal { .. })
        })
        .with_context(reciprocal_ctx)?;

        self.validate_agents(&[agent_1_id, agent_2_id])
            .with_context(reciprocal_ctx)?;

        let agent_1 = self.agents.get(agent_1_id).unwrap();

        // Order doesn't matter, but still go off of one arbitrary agent's
        // edges for lookup, meaning we don't have to scan all relations.
        Ok(agent_1.edges.iter().find_map(|edge| {
            if edge.relation_type == type_id {
                if let Some(relation) = self.relation_store.get(edge.relation_handle.clone()) {
                    if let RelationData::Reciprocal {
                        agent_1: a1,
                        agent_2: a2,
                    } = &relation.data
                    {
                        if (a1 == agent_1_id && a2 == agent_2_id)
                            || (a1 == agent_2_id && a2 == agent_1_id)
                        {
                            return Some((edge, relation));
                        }
                    }
                }
            }
            None
        }))
    }

    /// Adds an evaluation to the world.
    ///
    /// An evaluation is a directed relation that connects two agents, but also
    /// includes a "reason" field that provides additional context for the
    /// relation. As with directional relations, the from agent is always at
    /// index `0` in the relation's edges list, and the to agent is always at
    /// index `1`.
    ///
    /// Returns an error if the type does not exist or is not an evaluation
    /// type, if either agent does not exist, or if the fields do not match the
    /// type definition. Otherwise, returns an associated `RelationCreated`
    /// containing a handle to the newly created relation and a list of
    /// mutations that were applied to the world as part of creating the
    /// relation.
    pub fn add_evaluation(
        &mut self,
        from_id: &str,
        to_id: &str,
        type_id: &str,
        fields: Fields,
        reason: &str,
    ) -> Result<RelationCreated> {
        let evaluation_ctx = || {
            format!(
                "adding evaluation {} from {} to {}",
                type_id, from_id, to_id
            )
        };

        self.expect_type(type_id, "evaluation", |d| {
            matches!(d, RelationTypeData::Evaluation { .. })
        })
        .with_context(evaluation_ctx)?;

        self.validate_type_fields(type_id, &fields)
            .with_context(evaluation_ctx)?;

        self.validate_agents(&[from_id, to_id])
            .with_context(evaluation_ctx)?;

        let mut created = self.add_relation(Relation {
            type_name: type_id.to_string(),
            edges: vec![
                RelationToAgent::Forward(from_id.to_string()),
                RelationToAgent::Backward(to_id.to_string()),
            ],
            fields,
            data: RelationData::Evaluation {
                agent_from: from_id.to_string(),
                agent_to: to_id.to_string(),
                reason: reason.to_string(),
            },
        });

        let from_agent = self.agents.get_mut(from_id).unwrap();

        created.mutations.push(WorldMutation::AgentEdgesUpdated {
            agent_id: from_id.to_string(),
            prior_edges: from_agent.edges.clone(),
        });

        from_agent.edges.push(AgentToRelation {
            index: 0,
            relation_type: type_id.to_string(),
            relation_handle: created.handle.clone(),
        });

        let to_agent = self.agents.get_mut(to_id).unwrap();

        created.mutations.push(WorldMutation::AgentEdgesUpdated {
            agent_id: to_id.to_string(),
            prior_edges: to_agent.edges.clone(),
        });

        to_agent.edges.push(AgentToRelation {
            index: 1,
            relation_type: type_id.to_string(),
            relation_handle: created.handle.clone(),
        });

        Ok(created)
    }

    /// Retrieves an evaluation relation for the given from and to agents and
    /// type, if it exists. Works on both primary and complement type names,
    /// but requires correct ordering of arguments, with the from agent as
    /// `from` and the to agent as `to`.
    ///
    /// Refer to `World::get_directional` for more details on how the lookup
    /// handles type complements and ordering of arguments.
    ///
    /// Returns an error if the type does not exist or is not an evaluation
    /// type, or if either of the agents do not exist. Otherwise, returns
    /// `Ok(None)` if the agents do not have that relation, and
    /// `Ok(Some((edge, relation)))` if they do.
    pub fn get_evaluation(
        &self,
        from_id: &str,
        to_id: &str,
        type_id: &str,
    ) -> Result<Option<(&AgentToRelation, &Relation)>> {
        // If the edge is a complement, perform a search on the primary type
        // with the from and to reversed. This allows us to find the edge
        // regardless of which direction it's referred to as.
        if let Some(primary_type_name) = self.relation_type_map.get_complement_entry(type_id) {
            return self.get_evaluation(to_id, from_id, primary_type_name);
        }

        // Error checking!
        let evaluation_ctx = || {
            format!(
                "getting evaluation {} from {} to {}",
                type_id, from_id, to_id
            )
        };

        self.expect_type(type_id, "evaluation", |d| {
            matches!(d, RelationTypeData::Evaluation { .. })
        })
        .with_context(evaluation_ctx)?;

        self.validate_agents(&[from_id, to_id])
            .with_context(evaluation_ctx)?;

        let from_agent = self.agents.get(from_id).unwrap();

        Ok(from_agent.edges.iter().find_map(|edge| {
            if edge.relation_type == type_id && edge.index == 0 {
                if let Some(relation) = self.relation_store.get(edge.relation_handle.clone()) {
                    if let RelationData::Evaluation { agent_to, .. } = &relation.data {
                        if agent_to == to_id {
                            return Some((edge, relation));
                        }
                    }
                }
            }
            None
        }))
    }

    /// Adds a practice to the world.
    ///
    /// A practice is a relation that connects multiple agents together around
    /// a shared functionality. The specific roles of the agents in the
    /// practice are determined by the parameters provided by the type
    /// definition, in conjunction with the arguments passed in. This makes
    /// practices ordering dependent, so a practice created with participants
    /// `[x, y]` will not be the same as a practice of the same type created
    /// with participants `[y, x]` and will not be retrieved if the latter is
    /// used in the corresponding lookup, `World::get_practice`.
    ///
    /// Returns an error if the type does not exist or is not a practice
    /// type, if any agent does not exist, or if the fields do not match the
    /// type definition. Otherwise, returns an associated `RelationCreated`
    /// containing a handle to the newly created relation and a list of
    /// mutations that were applied to the world as part of creating the
    /// relation.
    pub fn add_practice(
        &mut self,
        participant_ids: Vec<&str>,
        type_id: &str,
        fields: Fields,
    ) -> Result<RelationCreated> {
        let practice_ctx = || {
            format!(
                "adding practice {} with participants {:?}",
                type_id, participant_ids
            )
        };

        self.expect_type(type_id, "practice", |d| {
            matches!(d, RelationTypeData::Practice { .. })
        })
        .with_context(practice_ctx)?;

        self.validate_type_fields(type_id, &fields)
            .with_context(practice_ctx)?;

        self.validate_agents(&participant_ids)
            .with_context(practice_ctx)?;

        let type_def = self
            .relation_type_map
            .get_type(type_id)
            .with_context(practice_ctx)?;

        let RelationTypeData::Practice { params, .. } = &type_def.data else {
            bail!("type {} is not a practice type", type_id);
        };

        if params.len() != participant_ids.len() {
            bail!(
                "practice type {} expects {} participants, but {} were provided",
                type_id,
                params.len(),
                participant_ids.len()
            );
        }

        let variables: HashMap<String, String> = params
            .iter()
            .cloned()
            .zip(participant_ids.iter().cloned().map(String::from))
            .collect();
        let mut self_id = vec!["practice".to_string()];
        self_id.push(type_id.to_string());
        self_id.extend(participant_ids.iter().cloned().map(String::from));
        let bindings = Bindings::new(variables, Some(self_id.into()));

        let edges = participant_ids
            .iter()
            .cloned()
            .map(|p| RelationToAgent::Ordered(p.into()))
            .collect();

        let mut created = self.add_relation(Relation {
            type_name: type_id.to_string(),
            edges,
            fields,
            data: RelationData::Practice {
                agents: participant_ids.iter().cloned().map(String::from).collect(),
                bindings,
            },
        });

        for (i, participant) in participant_ids.iter().enumerate() {
            let agent = self.agents.get_mut(*participant).unwrap();

            created.mutations.push(WorldMutation::AgentEdgesUpdated {
                agent_id: participant.to_string(),
                prior_edges: agent.edges.clone(),
            });

            agent.edges.push(AgentToRelation {
                index: i,
                relation_type: type_id.to_string(),
                relation_handle: created.handle.clone(),
            });
        }

        Ok(created)
    }

    /// Retrieves a practice relation for the given participants and type, if
    /// it exists. The participants must be in the same order as they were when
    /// the practice was created, since the ordering of agents in a practice is
    /// significant for determining their roles in the practice.
    ///
    /// Returns an error if the type does not exist or is not a practice type,
    /// or if any of the agents do not exist. Otherwise, returns `Ok(None)` if
    /// the agents do not have that relation, and `Ok(Some((edge, relation)))`
    /// if they do.
    pub fn get_practice(
        &self,
        participant_ids: Vec<&str>,
        type_id: &str,
    ) -> Result<Option<(&AgentToRelation, &Relation)>> {
        // Error checking!
        let practice_ctx = || {
            format!(
                "getting practice {} with participants {:?}",
                type_id, participant_ids
            )
        };

        self.expect_type(type_id, "practice", |d| {
            matches!(d, RelationTypeData::Practice { .. })
        })
        .with_context(practice_ctx)?;

        // More error checking! Make sure all participants exist

        self.validate_agents(&participant_ids)
            .with_context(practice_ctx)?;

        // Should never fail but blehhhh...
        let agent_1 = self.agents.get(participant_ids[0]).with_context(|| {
            format!(
                "looking up first participant {} for practice retrieval",
                participant_ids[0]
            )
        })?;

        // Work off of the arbitrary first agent for faster lookup
        Ok(agent_1.edges.iter().find_map(|edge| {
            // We know this is the first agent so we know it's at index 0
            if edge.relation_type == type_id && edge.index == 0 {
                if let Some(relation) = self.relation_store.get(edge.relation_handle.clone()) {
                    // Participants must match exactly, since order matters for practices
                    if relation
                        .edges
                        .iter()
                        // Edges are ordered on creation, so direct
                        // comparison works
                        .map(|e| e.agent())
                        .eq(participant_ids.iter().cloned())
                    {
                        return Some((edge, relation));
                    }
                }
            }
            None
        }))
    }

    /// Adds a binary relation between two agents, with the specific edge type
    /// determined by the type mapping.
    ///
    /// This function can exist because the inputs for all binary relation
    /// types are the same, so we can determine the specific type of relation
    /// to create dynamically based on the type mapping entry for the provided
    /// edge type name. This allows us to have a single function for adding
    /// them, as is reflected in the ambiguous `x.relation.y` syntax.
    ///
    /// Returns an error if the type does not exist or is not a supported
    /// binary relation type (i.e. directional, reciprocal, or evaluation), if
    /// either agent does not exist, if the fields do not match the type
    /// definition, or if the required "reason" field is not provided for
    /// evaluation types. Otherwise, returns an associated `RelationCreated`
    /// containing a handle to the newly created relation and a list of
    /// mutations that were applied to the world as part of creating the
    /// relation.
    pub fn add_binary_relation(
        &mut self,
        from_id: &str,
        to_id: &str,
        edge_type_id: &str,
        mut fields: Fields,
    ) -> Result<RelationCreated> {
        let edge_type = self
            .relation_type_map
            .get_type(edge_type_id)
            .with_context(|| {
                format!(
                    "looking up edge type {} for binary relation {} -> {}",
                    edge_type_id, from_id, to_id
                )
            })?;
        match edge_type.data {
            RelationTypeData::Directional { .. } => {
                self.add_directional(from_id, to_id, edge_type_id, fields)
            }
            RelationTypeData::Reciprocal => {
                self.add_reciprocal(from_id, to_id, edge_type_id, fields)
            }
            RelationTypeData::Evaluation { .. } => {
                let reason = fields
                    .get_mut("reason")
                    .context("evaluation edges require a 'reason' field")?;
                let Constant::String(reason_str) = reason else {
                    bail!("evaluation edge 'reason' field must be a string");
                };
                // TODO: Definitely some way to avoid this clone...
                let reason_string = reason_str.clone();
                fields.remove("reason");
                self.add_evaluation(from_id, to_id, edge_type_id, fields, &reason_string)
            }
            _ => bail!(
                "edge type {} has unsupported variant {:?} for bidirectional declaration",
                edge_type_id,
                edge_type.data
            ),
        }
    }

    /// Gets a binary relation (i.e. directional, reciprocal, or evaluation)
    /// between two agents, with the specific edge type determined by the type
    /// mapping. Works on both primary and complement type names, but requires
    /// correct ordering of arguments if relevant.
    ///
    /// Returns an error if the type does not exist or is not a supported
    /// binary relation type (i.e. directional, reciprocal, or evaluation), or
    /// if either agent does not exist. Otherwise, returns `Ok(None)` if the
    /// agents do not have that relation, and `Ok(Some((edge, relation)))` if
    /// they do.
    pub fn get_binary_relation(
        &self,
        from: &str,
        to: &str,
        edge_type_name: &str,
    ) -> Result<Option<(&AgentToRelation, &Relation)>> {
        match self.relation_type_map.get_type(edge_type_name) {
            Some(edge_type) => match edge_type.data {
                RelationTypeData::Directional { .. } => {
                    self.get_directional(from, to, edge_type_name)
                }
                RelationTypeData::Reciprocal => self.get_reciprocal(from, to, edge_type_name),
                RelationTypeData::Evaluation { .. } => {
                    self.get_evaluation(from, to, edge_type_name)
                }
                _ => bail!(
                    "edge type {} has unsupported variant {:?} for binary relation retrieval",
                    edge_type_name,
                    edge_type.data
                ),
            },
            None => bail!(
                "edge type {} does not exist for binary relation retrieval",
                edge_type_name
            ),
        }
    }

    pub fn undo_mutation(&mut self, mutation: WorldMutation) -> Result<()> {
        match mutation {
            WorldMutation::RelationAdded { handle } => self.relation_store.remove(handle),
            WorldMutation::RelationRemoved { handle, prior } => {
                self.relation_store.restore(handle, prior)
            }
            WorldMutation::RelationFieldsUpdated {
                handle,
                prior_fields,
            } => {
                let cloned_handle = handle.clone();
                let relation = self.relation_store.get_mut(handle).with_context(|| {
                    format!(
                        "looking up relation {:?} for undoing field update",
                        cloned_handle
                    )
                })?;
                relation.fields = prior_fields;
                Ok(())
            }
            WorldMutation::AgentSetActive {
                agent_id,
                prior_active,
            } => {
                let agent = self.agents.get_mut(&agent_id).with_context(|| {
                    format!(
                        "looking up agent {} for undoing active state change",
                        agent_id
                    )
                })?;
                agent.is_active = prior_active;
                Ok(())
            }
            WorldMutation::AgentEdgesUpdated {
                agent_id,
                prior_edges,
            } => {
                let agent = self.agents.get_mut(&agent_id).with_context(|| {
                    format!("looking up agent {} for undoing edges update", agent_id)
                })?;
                agent.edges = prior_edges;
                Ok(())
            }
            WorldMutation::AgentEmotionUpdated {
                agent_id,
                prior_emotion,
            } => {
                let agent = self.agents.get_mut(&agent_id).with_context(|| {
                    format!("looking up agent {} for undoing emotion update", agent_id)
                })?;
                agent.emotion = prior_emotion;
                Ok(())
            }
        }
    }
}
