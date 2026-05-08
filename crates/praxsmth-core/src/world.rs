use std::collections::HashMap;

use anyhow::{Context, Result, bail};

use crate::{
    definitions::{
        FieldTypes, PraxsmthConstant, PraxsmthField, Sentence, Serialize, types::PraxsmthTypeData,
        world::*,
    },
    types::TypeMapping,
};

pub mod api;
pub mod simulation;

type Fields = HashMap<String, PraxsmthConstant>;

// TODO: verify this works correctly in all cases, and add more detailed error messages
fn verify_fields(fields: &Fields, field_types: &FieldTypes, require_all: bool) -> Result<()> {
    if require_all {
        for field_name in field_types.keys() {
            if !fields.contains_key(field_name) {
                bail!("field {} is required but not present", field_name);
            }
        }
    }
    for (field_name, field_value) in fields {
        match field_types.get(field_name) {
            Some(expected_type) => match (expected_type, field_value) {
                (PraxsmthField::NumberRange(start, end), PraxsmthConstant::Number(n)) => {
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
                (PraxsmthField::VariantList(variants), PraxsmthConstant::Variant(v)) => {
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
                        expected_type.serialize(),
                        field_value.serialize()
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

#[derive(Debug, Clone)]
pub struct Bindings {
    variables: HashMap<String, String>,
    self_id: Option<Sentence>,
}

impl Bindings {
    pub fn new(variables: HashMap<String, String>, self_id: Option<Sentence>) -> Self {
        Bindings { variables, self_id }
    }

    pub fn get(&self, var: &str) -> Option<&String> {
        self.variables.get(var)
    }

    pub fn insert(&mut self, var: String, value: String) {
        self.variables.insert(var, value);
    }
}

impl Default for Bindings {
    fn default() -> Self {
        Bindings {
            variables: HashMap::new(),
            self_id: None,
        }
    }
}

impl<'a> IntoIterator for &'a Bindings {
    type Item = (&'a String, &'a String);
    type IntoIter = std::collections::hash_map::Iter<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.variables.iter()
    }
}

#[derive(Debug, Clone)]
pub enum AgentToRelation {
    Trait(RelationHandle),
    Emotion(RelationHandle),
    DirectionalForward(RelationHandle),
    DirectionalBackward(RelationHandle),
    Reciprocal(RelationHandle),
    EvaluationForward(RelationHandle),
    EvaluationBackward(RelationHandle),
    Practice(RelationHandle),
}

impl AgentToRelation {
    pub fn handle(&self) -> RelationHandle {
        match self {
            AgentToRelation::Trait(h)
            | AgentToRelation::Emotion(h)
            | AgentToRelation::DirectionalForward(h)
            | AgentToRelation::DirectionalBackward(h)
            | AgentToRelation::Reciprocal(h)
            | AgentToRelation::EvaluationForward(h)
            | AgentToRelation::EvaluationBackward(h)
            | AgentToRelation::Practice(h) => h.clone(),
        }
    }
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
    Trait,
    Directional,
    Reciprocal,
    Evaluation { reason: String },
    Emotion,
    Practice { bindings: Bindings },
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

pub struct Agent {
    pub name: String,
    pub edges: Vec<AgentToRelation>,
    // Quick access field for the singular emotion they might have
    pub emotion: Option<RelationHandle>,
}

impl Agent {
    pub fn new(info: &AgentInfo) -> Self {
        // TODO: better agent construction
        Agent {
            name: info.name.clone(),
            edges: Vec::new(),
            emotion: None,
        }
    }

    pub fn remove_edges_to(&mut self, handle: RelationHandle) {
        self.edges.retain(|edge| edge.handle() != handle);
    }
}

pub struct World {
    pub agents: HashMap<String, Agent>,
    pub type_mapping: TypeMapping,
    pub relation_store: RelationStore,
}

impl World {
    pub fn new(type_mapping: TypeMapping) -> Self {
        World {
            agents: HashMap::new(),
            type_mapping,
            relation_store: RelationStore::new(),
        }
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

    pub fn add_agent(&mut self, info: &AgentInfo) -> Result<()> {
        if self.agents.contains_key(&info.id) {
            bail!("agent with id {} already exists", info.id);
        }
        self.agents.insert(info.id.clone(), Agent::new(info));
        Ok(())
    }

    pub fn get_agent(&self, name: &str) -> Option<&Agent> {
        self.agents.get(name)
    }

    pub fn get_relation(&self, handle: RelationHandle) -> Option<&Relation> {
        self.relation_store.get(handle)
    }

    pub fn add_relation(&mut self, edge: Relation) -> RelationHandle {
        self.relation_store.add(edge)
    }

    pub fn update_relation(&mut self, handle: RelationHandle, new_fields: Fields) -> Result<()> {
        let relation = self
            .relation_store
            .get_mut(handle.clone())
            .with_context(|| format!("looking up relation {:?} for update", handle))?;
        let relation_type = self
            .type_mapping
            .get_type(&relation.type_name)
            .with_context(|| {
                format!(
                    "looking up type {} for relation {:?}",
                    relation.type_name, handle
                )
            })?;
        match &mut relation.data {
            RelationData::Trait
            | RelationData::Emotion
            | RelationData::Directional
            | RelationData::Reciprocal
            | RelationData::Practice { .. } => relation
                .update_fields(new_fields, &relation_type.fields)
                .with_context(|| format!("updating fields on relation {:?}", handle)),
            RelationData::Evaluation { reason } => {
                if let Some(new_reason) = new_fields.get("reason") {
                    let PraxsmthConstant::String(reason_str) = new_reason else {
                        bail!("evaluation edge 'reason' field must be a string");
                    };
                    *reason = reason_str.clone();
                }
                relation
                    .update_fields(new_fields, &relation_type.fields)
                    .with_context(|| format!("updating fields on evaluation relation {:?}", handle))
            }
        }
    }

    pub fn remove_relation(&mut self, handle: RelationHandle) -> Result<()> {
        let relation = self
            .relation_store
            .get(handle.clone())
            .with_context(|| format!("looking up relation {:?} for removal", handle))?;
        relation.edges.iter().for_each(|edge_to_agent| {
            let agent_name = edge_to_agent.agent();
            if let Some(agent) = self.agents.get_mut(agent_name) {
                agent.remove_edges_to(handle.clone());
            } else {
                panic!(
                    "agent with name {} not found when removing relation with handle {:?}",
                    agent_name, handle
                );
            }
        });
        self.relation_store
            .remove(handle.clone())
            .with_context(|| format!("removing relation {:?} from store", handle))
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
            .type_mapping
            .get_type(type_name)
            .with_context(|| format!("looking up type {} in type mapping", type_name))?;
        verify_fields(fields, &edge_type.fields, true)
            .with_context(|| format!("verifying fields against type {}", type_name))
    }

    pub fn add_trait(
        &mut self,
        agent: &str,
        type_name: &str,
        fields: Fields,
    ) -> Result<RelationHandle> {
        self.validate_type_fields(type_name, &fields)
            .with_context(|| format!("adding trait {} to agent {}", type_name, agent))?;
        self.validate_agent(agent)
            .with_context(|| format!("adding trait {} to agent {}", type_name, agent))?;

        let handle = self.add_relation(Relation {
            type_name: type_name.to_string(),
            edges: vec![RelationToAgent::Solo(agent.to_string())],
            fields,
            data: RelationData::Trait,
        });

        self.agents
            .get_mut(agent)
            .unwrap()
            .edges
            .push(AgentToRelation::Trait(handle.clone()));

        Ok(handle)
    }

    pub fn get_trait(&self, agent: &str, type_name: &str) -> Option<(RelationHandle, &Relation)> {
        self.agents.get(agent)?.edges.iter().find_map(|edge| {
            if let AgentToRelation::Trait(handle) = edge {
                if let Some(relation) = self.relation_store.get(handle.clone()) {
                    if relation.type_name == type_name {
                        return Some((handle.clone(), relation));
                    }
                }
            }
            None
        })
    }

    pub fn add_emotion(
        &mut self,
        agent: &str,
        type_name: &str,
        fields: Fields,
    ) -> Result<RelationHandle> {
        self.validate_type_fields(type_name, &fields)
            .with_context(|| format!("adding emotion {} to agent {}", type_name, agent))?;
        self.validate_agent(agent)
            .with_context(|| format!("adding emotion {} to agent {}", type_name, agent))?;

        let handle = self.add_relation(Relation {
            type_name: type_name.to_string(),
            edges: vec![RelationToAgent::Solo(agent.to_string())],
            fields,
            data: RelationData::Emotion,
        });

        let agent_data = self.agents.get_mut(agent).with_context(|| {
            format!(
                "agent {} disappeared between validation and emotion edge insertion",
                agent
            )
        })?;

        let old_emotion_handle = agent_data.emotion.clone();

        agent_data
            .edges
            .push(AgentToRelation::Emotion(handle.clone()));
        agent_data.emotion = Some(handle.clone());

        // Remove the old emotion edge for this agent, since an agent can only have one emotion edge at a time
        if let Some(old_emotion_handle) = old_emotion_handle {
            self.remove_relation(old_emotion_handle)
                .with_context(|| format!("replacing prior emotion edge on agent {}", agent))?;
        }

        Ok(handle)
    }

    pub fn get_emotion(&self, agent: &str, type_name: &str) -> Option<(RelationHandle, &Relation)> {
        self.agents.get(agent)?.edges.iter().find_map(|edge| {
            if let AgentToRelation::Emotion(handle) = edge {
                if let Some(relation) = self.relation_store.get(handle.clone()) {
                    if relation.type_name == type_name {
                        return Some((handle.clone(), relation));
                    }
                }
            }
            None
        })
    }

    pub fn add_directional(
        &mut self,
        from: &str,
        to: &str,
        type_name: &str,
        fields: Fields,
    ) -> Result<RelationHandle> {
        self.validate_type_fields(type_name, &fields)
            .with_context(|| format!("adding directional {} from {} to {}", type_name, from, to))?;
        self.validate_agents(&[from, to])
            .with_context(|| format!("adding directional {} from {} to {}", type_name, from, to))?;

        let handle = self.add_relation(Relation {
            type_name: type_name.to_string(),
            edges: vec![
                RelationToAgent::Forward(from.to_string()),
                RelationToAgent::Backward(to.to_string()),
            ],
            fields,
            data: RelationData::Directional,
        });

        self.agents
            .get_mut(from)
            .unwrap()
            .edges
            .push(AgentToRelation::DirectionalForward(handle.clone()));

        self.agents
            .get_mut(to)
            .unwrap()
            .edges
            .push(AgentToRelation::DirectionalBackward(handle.clone()));

        Ok(handle)
    }

    pub fn get_directional(
        &self,
        from: &str,
        to: &str,
        type_name: &str,
    ) -> Option<(RelationHandle, &Relation)> {
        self.agents.get(from)?.edges.iter().find_map(|edge| {
            if let AgentToRelation::DirectionalForward(handle) = edge {
                if let Some(relation) = self.relation_store.get(handle.clone()) {
                    if relation.type_name == type_name {
                        // check that the other edge matches the expected to agent
                        if relation
                            .edges
                            .iter()
                            .any(|e| matches!(e, RelationToAgent::Backward(a) if a == to))
                        {
                            return Some((handle.clone(), relation));
                        }
                    }
                }
            }
            // Check backwards edges too!
            if let AgentToRelation::DirectionalBackward(handle) = edge {
                if let Some(relation) = self.relation_store.get(handle.clone()) {
                    if relation.type_name == type_name {
                        // check that the other edge matches the expected from agent
                        if relation
                            .edges
                            .iter()
                            .any(|e| matches!(e, RelationToAgent::Forward(a) if a == from))
                        {
                            return Some((handle.clone(), relation));
                        }
                    }
                }
            }
            None
        })
    }

    pub fn add_reciprocal(
        &mut self,
        agent_1: &str,
        agent_2: &str,
        type_name: &str,
        fields: Fields,
    ) -> Result<RelationHandle> {
        self.validate_type_fields(type_name, &fields)
            .with_context(|| {
                format!(
                    "adding reciprocal {} between {} and {}",
                    type_name, agent_1, agent_2
                )
            })?;
        self.validate_agents(&[agent_1, agent_2]).with_context(|| {
            format!(
                "adding reciprocal {} between {} and {}",
                type_name, agent_1, agent_2
            )
        })?;

        let handle = self.add_relation(Relation {
            type_name: type_name.to_string(),
            edges: vec![
                RelationToAgent::Unordered(agent_1.to_string()),
                RelationToAgent::Unordered(agent_2.to_string()),
            ],
            fields,
            data: RelationData::Reciprocal,
        });

        self.agents
            .get_mut(agent_1)
            .unwrap()
            .edges
            .push(AgentToRelation::Reciprocal(handle.clone()));

        self.agents
            .get_mut(agent_2)
            .unwrap()
            .edges
            .push(AgentToRelation::Reciprocal(handle.clone()));

        Ok(handle)
    }

    pub fn get_reciprocal(
        &self,
        agent_1: &str,
        agent_2: &str,
        type_name: &str,
    ) -> Option<(RelationHandle, &Relation)> {
        self.agents.get(agent_1)?.edges.iter().find_map(|edge| {
            if let AgentToRelation::Reciprocal(handle) = edge {
                if let Some(relation) = self.relation_store.get(handle.clone()) {
                    if relation.type_name == type_name {
                        // check that the other edge matches the expected second agent
                        if relation
                            .edges
                            .iter()
                            .any(|e| matches!(e, RelationToAgent::Unordered(a) if a == agent_2))
                        {
                            return Some((handle.clone(), relation));
                        }
                    }
                }
            }
            None
        })
    }

    pub fn add_evaluation(
        &mut self,
        from: &str,
        to: &str,
        type_name: &str,
        fields: Fields,
        reason: &str,
    ) -> Result<RelationHandle> {
        self.validate_type_fields(type_name, &fields)
            .with_context(|| format!("adding evaluation {} from {} to {}", type_name, from, to))?;
        self.validate_agents(&[from, to])
            .with_context(|| format!("adding evaluation {} from {} to {}", type_name, from, to))?;

        let handle = self.add_relation(Relation {
            type_name: type_name.to_string(),
            edges: vec![
                RelationToAgent::Forward(from.to_string()),
                RelationToAgent::Backward(to.to_string()),
            ],
            fields,
            data: RelationData::Evaluation {
                reason: reason.to_string(),
            },
        });

        self.agents
            .get_mut(from)
            .unwrap()
            .edges
            .push(AgentToRelation::EvaluationForward(handle.clone()));

        self.agents
            .get_mut(to)
            .unwrap()
            .edges
            .push(AgentToRelation::EvaluationBackward(handle.clone()));

        Ok(handle)
    }

    pub fn get_evaluation(
        &self,
        from: &str,
        to: &str,
        type_name: &str,
    ) -> Option<(RelationHandle, &Relation)> {
        self.agents.get(from)?.edges.iter().find_map(|edge| {
            if let AgentToRelation::EvaluationForward(handle) = edge {
                if let Some(relation) = self.relation_store.get(handle.clone()) {
                    if relation.type_name == type_name {
                        // check that the other edge matches the expected to agent
                        if relation
                            .edges
                            .iter()
                            .any(|e| matches!(e, RelationToAgent::Backward(a) if a == to))
                        {
                            return Some((handle.clone(), relation));
                        }
                    }
                }
            }
            // Check backwards edges too!
            if let AgentToRelation::EvaluationBackward(handle) = edge {
                if let Some(relation) = self.relation_store.get(handle.clone()) {
                    if relation.type_name == type_name {
                        // check that the other edge matches the expected from agent
                        if relation
                            .edges
                            .iter()
                            .any(|e| matches!(e, RelationToAgent::Forward(a) if a == from))
                        {
                            return Some((handle.clone(), relation));
                        }
                    }
                }
            }
            None
        })
    }

    pub fn add_practice(
        &mut self,
        participants: Vec<String>,
        type_name: &str,
        fields: Fields,
    ) -> Result<RelationHandle> {
        let practice_ctx = || {
            format!(
                "adding practice {} with participants {:?}",
                type_name, participants
            )
        };

        self.validate_type_fields(type_name, &fields)
            .with_context(practice_ctx)?;

        let participant_refs: Vec<&str> = participants.iter().map(|s| s.as_str()).collect();
        self.validate_agents(&participant_refs)
            .with_context(practice_ctx)?;

        let type_def = self
            .type_mapping
            .get_type(type_name)
            .with_context(practice_ctx)?;

        let PraxsmthTypeData::Practice { params, .. } = &type_def.data else {
            bail!("type {} is not a practice type", type_name);
        };

        if params.len() != participants.len() {
            bail!(
                "practice type {} expects {} participants, but {} were provided",
                type_name,
                params.len(),
                participants.len()
            );
        }

        let variables: HashMap<String, String> = params
            .iter()
            .cloned()
            .zip(participants.iter().cloned())
            .collect();
        let mut self_id = vec!["practice".to_string()];
        self_id.push(type_name.to_string());
        self_id.extend(participants.iter().cloned());
        let bindings = Bindings::new(variables, Some(self_id));

        let edges = participants
            .iter()
            .map(|p| RelationToAgent::Ordered(p.clone()))
            .collect();

        let handle = self.add_relation(Relation {
            type_name: type_name.to_string(),
            edges,
            fields,
            data: RelationData::Practice { bindings },
        });

        for participant in participants {
            self.agents
                .get_mut(&participant)
                .unwrap()
                .edges
                .push(AgentToRelation::Practice(handle.clone()));
        }

        Ok(handle)
    }

    pub fn get_practice(
        &self,
        participants: Vec<String>,
        type_name: &str,
    ) -> Option<(RelationHandle, &Relation)> {
        self.agents
            .get(&participants[0])?
            .edges
            .iter()
            .find_map(|edge| {
                if let AgentToRelation::Practice(handle) = edge {
                    if let Some(relation) = self.relation_store.get(handle.clone()) {
                        if relation.type_name == type_name {
                            // participants must match exactly, since order matters for practices
                            if relation
                                .edges
                                .iter()
                                // Assume all edges are ordered
                                .map(|e| e.agent())
                                .eq(participants.iter())
                            {
                                return Some((handle.clone(), relation));
                            }
                        }
                    }
                }
                None
            })
    }

    /// Adds a binary relation between two agents, with the specific edge type determined by the type mapping.
    pub fn add_binary_relation(
        &mut self,
        from: &str,
        to: &str,
        edge_type_name: &str,
        mut fields: Fields,
    ) -> Result<RelationHandle> {
        let edge_type = self
            .type_mapping
            .get_type(edge_type_name)
            .with_context(|| {
                format!(
                    "looking up edge type {} for binary relation {} -> {}",
                    edge_type_name, from, to
                )
            })?;
        match edge_type.data {
            PraxsmthTypeData::Directional { .. } => {
                self.add_directional(from, to, edge_type_name, fields)
            }
            PraxsmthTypeData::Reciprocal => self.add_reciprocal(from, to, edge_type_name, fields),
            PraxsmthTypeData::Evaluation { .. } => {
                let reason = fields
                    .get_mut("reason")
                    .context("evaluation edges require a 'reason' field")?;
                let PraxsmthConstant::String(reason_str) = reason else {
                    bail!("evaluation edge 'reason' field must be a string");
                };
                // TODO: Definitely some way to avoid this clone...
                let reason_string = reason_str.clone();
                fields.remove("reason");
                self.add_evaluation(from, to, edge_type_name, fields, &reason_string)
            }
            _ => bail!(
                "edge type {} has unsupported variant {:?} for bidirectional declaration",
                edge_type_name,
                edge_type.data
            ),
        }
    }

    /// // TODO: More descriptive errors for all get functions
    /// Gets a binary relation between two agents, regardless of the specific edge type, as long as it matches the expected type variant in the type mapping.
    pub fn get_binary_relation(
        &self,
        from: &str,
        to: &str,
        edge_type_name: &str,
    ) -> Option<(RelationHandle, &Relation)> {
        match self.type_mapping.get_type(edge_type_name) {
            Some(edge_type) => match edge_type.data {
                PraxsmthTypeData::Directional { .. } => {
                    self.get_directional(from, to, edge_type_name)
                }
                PraxsmthTypeData::Reciprocal => self.get_reciprocal(from, to, edge_type_name),
                PraxsmthTypeData::Evaluation { .. } => {
                    self.get_evaluation(from, to, edge_type_name)
                }
                _ => None,
            },
            None => None,
        }
    }
}
