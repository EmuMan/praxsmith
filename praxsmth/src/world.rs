use std::collections::HashMap;

use crate::{
    definitions::{
        PraxsmthConstant, PraxsmthField, Serialize, TypeFields, types::PraxsmthTypeData, world::*,
    },
    types::TypeMapping,
};

pub mod interface;

// TODO: verify this works correctly in all cases, and add more detailed error messages
fn verify_fields(
    fields: &HashMap<String, PraxsmthConstant>,
    field_types: &TypeFields,
    require_all: bool,
) -> Result<(), String> {
    if require_all {
        for field_name in field_types.keys() {
            if !fields.contains_key(field_name) {
                return Err(format!("Field {} is required but not present", field_name));
            }
        }
    }
    for (field_name, field_value) in fields {
        match field_types.get(field_name) {
            Some(expected_type) => match (expected_type, field_value) {
                (PraxsmthField::NumberRange(start, end), PraxsmthConstant::Number(n)) => {
                    if n < start || n > end {
                        return Err(format!(
                            "Field {} value {} is out of range {}..{}",
                            field_name, n, start, end
                        ));
                    }
                }
                (PraxsmthField::VariantList(variants), PraxsmthConstant::Variant(v)) => {
                    if !variants.contains(v) {
                        return Err(format!(
                            "Field {} value {} is not in variant list {:?}",
                            field_name, v, variants
                        ));
                    }
                }
                _ => {
                    return Err(format!(
                        "Field {} has type mismatch: expected {}, got {}",
                        field_name,
                        expected_type.serialize(),
                        field_value.serialize()
                    ));
                }
            },
            None => {
                return Err(format!("Field {} is not defined in type", field_name));
            }
        }
    }
    Ok(())
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
    type_name: String,
    edges: Vec<RelationToAgent>,
    fields: HashMap<String, PraxsmthConstant>,
    data: RelationData,
}

impl Relation {
    pub fn update_fields(
        &mut self,
        new_fields: HashMap<String, PraxsmthConstant>,
        field_defs: &HashMap<String, PraxsmthField>,
    ) -> Result<(), String> {
        verify_fields(&new_fields, &field_defs, false)?;
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
    Practice,
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

    pub fn remove(&mut self, handle: RelationHandle) -> Result<(), String> {
        if let Some(slot) = self.slots.get_mut(handle.index as usize) {
            if slot.generation == handle.generation {
                slot.value = None;
                slot.generation += 1;
                self.open_indices.push(handle.index as usize);
                Ok(())
            } else {
                Err("Invalid handle generation".to_string())
            }
        } else {
            Err("Invalid handle index".to_string())
        }
    }
}

pub struct Agent {
    pub edges: Vec<AgentToRelation>,
}

impl Agent {
    pub fn new(_info: AgentInfo) -> Self {
        // TODO: better agent construction
        Agent { edges: Vec::new() }
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

    pub fn add_agent(&mut self, info: AgentInfo) -> Result<(), String> {
        if self.agents.contains_key(&info.name) {
            return Err(format!("Agent with name {} already exists", info.name));
        }
        self.agents.insert(info.name.clone(), Agent::new(info));
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

    pub fn update_relation(
        &mut self,
        handle: RelationHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        match self.relation_store.get_mut(handle.clone()) {
            Some(relation) => {
                let Some(relation_type) = self.type_mapping.get_type(&relation.type_name) else {
                    return Err(format!(
                        "Type {} not found in type mapping for relation with handle {:?}",
                        relation.type_name, handle
                    ));
                };
                match &mut relation.data {
                    RelationData::Trait
                    | RelationData::Emotion
                    | RelationData::Directional
                    | RelationData::Reciprocal
                    | RelationData::Practice => {
                        relation.update_fields(new_fields, &relation_type.fields)
                    }
                    RelationData::Evaluation { reason } => {
                        if let Some(new_reason) = new_fields.get("reason") {
                            if let PraxsmthConstant::String(reason_str) = new_reason {
                                *reason = reason_str.clone();
                            } else {
                                return Err(
                                    "Evaluation edge 'reason' field must be a string".to_string()
                                );
                            }
                        }
                        relation.update_fields(new_fields, &relation_type.fields)
                    }
                }
            }
            None => Err(format!(
                "Relation with handle {:?} not found for update",
                handle
            )),
        }
    }

    pub fn remove_relation(&mut self, handle: RelationHandle) -> Result<(), String> {
        match self.relation_store.get(handle.clone()) {
            Some(relation) => {
                relation.edges.iter().for_each(|edge_to_agent| {
                    let agent_name = edge_to_agent.agent();
                    if let Some(agent) = self.agents.get_mut(agent_name) {
                        agent.remove_edges_to(handle.clone());
                    } else {
                        panic!(
                            "Agent with name {} not found when removing relation with handle {:?}",
                            agent_name, handle
                        );
                    }
                });
            }
            None => {
                return Err(format!("Relation handle {:?} not found", handle));
            }
        }
        self.relation_store.remove(handle)
    }

    fn validate_agent(&self, name: &str) -> Result<(), String> {
        if self.agents.contains_key(name) {
            Ok(())
        } else {
            Err(format!("Agent with name {} not found", name))
        }
    }

    fn validate_agents(&self, names: &[&str]) -> Result<(), String> {
        for name in names {
            self.validate_agent(name)?;
        }
        Ok(())
    }

    fn validate_type_fields(
        &self,
        type_name: &str,
        fields: &HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        match self.type_mapping.get_type(type_name) {
            Some(edge_type) => verify_fields(fields, &edge_type.fields, true),
            None => Err(format!("Type {} not found in type mapping", type_name)),
        }
    }

    pub fn add_trait(
        &mut self,
        agent: &str,
        type_name: &str,
        fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<RelationHandle, String> {
        self.validate_type_fields(type_name, &fields)?;

        self.validate_agent(agent)?;

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
        fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<RelationHandle, String> {
        self.validate_type_fields(type_name, &fields)?;

        self.validate_agent(agent)?;

        let handle = self.add_relation(Relation {
            type_name: type_name.to_string(),
            edges: vec![RelationToAgent::Solo(agent.to_string())],
            fields,
            data: RelationData::Emotion,
        });

        if let Some(agent_data) = self.agents.get_mut(agent) {
            // Remove any existing emotion edges for this agent, since an agent can only have one emotion edge at a time
            let existing_emotions: Vec<RelationHandle> = agent_data
                .edges
                .iter()
                .filter_map(|edge| match edge {
                    AgentToRelation::Emotion(h) => Some(h.clone()),
                    _ => None,
                })
                .collect();
            agent_data
                .edges
                .push(AgentToRelation::Emotion(handle.clone()));
            for emotion_handle in existing_emotions {
                self.remove_relation(emotion_handle)?;
            }
            Ok(handle)
        } else {
            panic!(
                "Agent with name {} not found when adding emotion edge",
                agent
            );
        }
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
        fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<RelationHandle, String> {
        self.validate_type_fields(type_name, &fields)?;

        self.validate_agents(&[from, to])?;

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
        fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<RelationHandle, String> {
        self.validate_type_fields(type_name, &fields)?;

        self.validate_agents(&[agent_1, agent_2])?;

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
        fields: HashMap<String, PraxsmthConstant>,
        reason: &str,
    ) -> Result<RelationHandle, String> {
        self.validate_type_fields(type_name, &fields)?;

        self.validate_agents(&[from, to])?;

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
        fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<RelationHandle, String> {
        self.validate_type_fields(type_name, &fields)?;

        let participant_refs: Vec<&str> = participants.iter().map(|s| s.as_str()).collect();
        self.validate_agents(&participant_refs)?;

        let edges = participants
            .iter()
            .map(|p| RelationToAgent::Ordered(p.clone()))
            .collect();

        let handle = self.add_relation(Relation {
            type_name: type_name.to_string(),
            edges,
            fields,
            data: RelationData::Practice,
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
        fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<RelationHandle, String> {
        match self.type_mapping.get_type(edge_type_name) {
            Some(edge_type) => match edge_type.data {
                PraxsmthTypeData::Directional { .. } => {
                    self.add_directional(from, to, edge_type_name, fields)
                }
                PraxsmthTypeData::Reciprocal => {
                    self.add_reciprocal(from, to, edge_type_name, fields)
                }
                PraxsmthTypeData::Evaluation { .. } => {
                    let reason = fields
                        .get("reason")
                        .ok_or_else(|| "Evaluation edges require a 'reason' field".to_string())?;
                    if let PraxsmthConstant::String(reason_str) = reason {
                        // TODO: Definitely some way to avoid this clone...
                        let reason_string = reason_str.clone();
                        self.add_evaluation(from, to, edge_type_name, fields, &reason_string)
                    } else {
                        return Err("Evaluation edge 'reason' field must be a string".to_string());
                    }
                }
                _ => Err(format!(
                    "Edge type {} has unsupported variant {:?} for bidirectional declaration",
                    edge_type_name, edge_type.data
                )),
            },
            None => Err(format!(
                "Edge type {} not found in type mapping for declaration",
                edge_type_name
            )),
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
