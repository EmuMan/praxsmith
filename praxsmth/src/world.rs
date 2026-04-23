use std::collections::HashMap;

use crate::{
    definitions::{
        PraxsmthConstant, PraxsmthField, Serialize, TypeFields, types::PraxsmthTypeData, world::*,
    },
    types::TypeMapping,
};

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
}

impl RelationToAgent {
    pub fn agent(&self) -> String {
        match self {
            RelationToAgent::Solo(a)
            | RelationToAgent::Forward(a)
            | RelationToAgent::Backward(a)
            | RelationToAgent::Unordered(a) => a.clone(),
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

    pub fn remove_relation(&mut self, handle: RelationHandle) -> Result<(), String> {
        match self.relation_store.get(handle.clone()) {
            Some(relation) => {
                relation.edges.iter().for_each(|edge_to_agent| {
                    let agent_name = edge_to_agent.agent();
                    if let Some(agent) = self.agents.get_mut(&agent_name) {
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

    /// Updates an edge's fields after `validate_data` confirms the edge variant is correct.
    fn update_relation(
        &mut self,
        handle: RelationHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
        validate_and_transform_data: impl FnOnce(&mut RelationData) -> Result<(), String>,
    ) -> Result<(), String> {
        match self.relation_store.get_mut(handle.clone()) {
            Some(edge) => {
                validate_and_transform_data(&mut edge.data)?;
                match self.type_mapping.get_type(&edge.type_name) {
                    Some(edge_type) => edge.update_fields(new_fields, &edge_type.fields),
                    None => Err(format!("Type {} not found in type mapping", edge.type_name)),
                }
            }
            None => Err(format!("Edge with handle {:?} not found", handle)),
        }
    }

    pub fn add_trait(
        &mut self,
        agent: &str,
        fields: HashMap<String, PraxsmthConstant>,
        type_name: &str,
    ) -> Result<RelationHandle, String> {
        self.validate_type_fields(type_name, &fields)?;

        if self.agents.get(agent).is_none() {
            return Err(format!("Agent with name {} not found", agent));
        }

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

    pub fn update_trait(
        &mut self,
        handle: RelationHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        self.update_relation(handle.clone(), new_fields, |data| match data {
            RelationData::Trait => Ok(()),
            _ => Err(format!(
                "Relation with handle {:?} is not a directional relation",
                handle
            )),
        })
    }

    pub fn add_emotion(
        &mut self,
        agent: &str,
        fields: HashMap<String, PraxsmthConstant>,
        type_name: &str,
    ) -> Result<RelationHandle, String> {
        self.validate_type_fields(type_name, &fields)?;

        if self.agents.get(agent).is_none() {
            return Err(format!("Agent with name {} not found", agent));
        }

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

    pub fn update_emotion(
        &mut self,
        handle: RelationHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        self.update_relation(handle.clone(), new_fields, |data| match data {
            RelationData::Emotion => Ok(()),
            _ => Err(format!(
                "Edge with handle {:?} is not an emotion edge",
                handle
            )),
        })
    }

    pub fn add_directional(
        &mut self,
        from: &str,
        to: &str,
        fields: HashMap<String, PraxsmthConstant>,
        type_name: &str,
    ) -> Result<RelationHandle, String> {
        self.validate_type_fields(type_name, &fields)?;

        if self.agents.get(from).is_none() {
            return Err(format!("Agent with name {} not found", from));
        }
        if self.agents.get(to).is_none() {
            return Err(format!("Agent with name {} not found", to));
        }

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

    pub fn update_directional(
        &mut self,
        handle: RelationHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        self.update_relation(handle.clone(), new_fields, |data| match data {
            RelationData::Directional => Ok(()),
            _ => Err(format!(
                "Relation with handle {:?} is not a directional relation",
                handle
            )),
        })
    }

    pub fn add_reciprocal(
        &mut self,
        agent_1: &str,
        agent_2: &str,
        fields: HashMap<String, PraxsmthConstant>,
        type_name: &str,
    ) -> Result<RelationHandle, String> {
        self.validate_type_fields(type_name, &fields)?;

        if self.agents.get(agent_1).is_none() {
            return Err(format!("Agent with name {} not found", agent_1));
        }
        if self.agents.get(agent_2).is_none() {
            return Err(format!("Agent with name {} not found", agent_2));
        }

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

    pub fn update_reciprocal(
        &mut self,
        handle: RelationHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        self.update_relation(handle.clone(), new_fields, |data| match data {
            RelationData::Reciprocal => Ok(()),
            _ => Err(format!(
                "Relation with handle {:?} is not a reciprocal relation",
                handle
            )),
        })
    }

    pub fn add_evaluation(
        &mut self,
        from: &str,
        to: &str,
        fields: HashMap<String, PraxsmthConstant>,
        type_name: &str,
        reason: &str,
    ) -> Result<RelationHandle, String> {
        self.validate_type_fields(type_name, &fields)?;

        if self.agents.get(from).is_none() {
            return Err(format!("Agent with name {} not found", from));
        }
        if self.agents.get(to).is_none() {
            return Err(format!("Agent with name {} not found", to));
        }

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

    pub fn update_evaluation(
        &mut self,
        handle: RelationHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
        new_reason: &str,
    ) -> Result<(), String> {
        self.update_relation(handle.clone(), new_fields, |data| match data {
            RelationData::Evaluation { reason } => {
                *reason = new_reason.to_string();
                Ok(())
            }
            _ => Err(format!(
                "Relation with handle {:?} is not an evaluation relation",
                handle
            )),
        })
    }

    pub fn add_practice(
        &mut self,
        participants: Vec<String>,
        fields: HashMap<String, PraxsmthConstant>,
        type_name: &str,
    ) -> Result<RelationHandle, String> {
        self.validate_type_fields(type_name, &fields)?;

        for participant in &participants {
            if self.agents.get(participant).is_none() {
                return Err(format!("Agent with name {} not found", participant));
            }
        }

        let edges = participants
            .iter()
            .map(|p| RelationToAgent::Unordered(p.clone()))
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

    pub fn process_declaration(&mut self, decl: Declaration) -> Result<(), String> {
        if decl.sentence.len() < 3 {
            return Err(format!(
                "Declaration sentence must have at least 3 parts: {:?}",
                decl.sentence.serialize()
            ));
        }

        match decl.sentence[0].as_str() {
            "practice" => {
                // Practice declaration: "practice.<practice_name>.<agent1>.<agent2>..."
                unimplemented!()
            }
            _ => (),
        }

        match decl.sentence[1].as_str() {
            "is" => {
                // Trait declaration: "<agent>.is.<trait>"
                let agent_name = &decl.sentence[0];
                let trait_name = &decl.sentence[2];

                self.add_trait(agent_name, decl.fields, trait_name)?;

                return Ok(());
            }
            "feels" => {
                // Emotion declaration: "<agent>.feels.<emotion>"
                let agent_name = &decl.sentence[0];
                let trait_name = &decl.sentence[2];

                self.add_emotion(agent_name, decl.fields, trait_name)?;

                return Ok(());
            }
            _ => (),
        }

        let edge_type_name = &decl.sentence[1];
        let from = &decl.sentence[0];
        let to = &decl.sentence[2];

        fn add_evaluation_helper(
            world: &mut World,
            from: &str,
            to: &str,
            fields: HashMap<String, PraxsmthConstant>,
            edge_type_name: &str,
        ) -> Result<(), String> {
            let reason = fields
                .get("reason")
                .ok_or_else(|| "Evaluation edges require a 'reason' field".to_string())?;
            if let PraxsmthConstant::String(reason_str) = reason {
                // TODO: Definitely some way to avoid this clone...
                let reason_string = reason_str.clone();
                world.add_evaluation(from, to, fields, edge_type_name, &reason_string)?;
                Ok(())
            } else {
                Err("Evaluation edge 'reason' field must be a string".to_string())
            }
        }

        match self.type_mapping.get_type(edge_type_name) {
            Some(edge_type) => match edge_type.data {
                PraxsmthTypeData::Directional { .. } => {
                    self.add_directional(from, to, decl.fields, edge_type_name)?;
                }
                PraxsmthTypeData::Reciprocal => {
                    self.add_reciprocal(from, to, decl.fields, edge_type_name)?;
                }
                PraxsmthTypeData::Evaluation { .. } => {
                    add_evaluation_helper(self, from, to, decl.fields, edge_type_name)?;
                }
                _ => {
                    return Err(format!(
                        "Edge type {} has unsupported variant {:?} for declaration {:?}",
                        edge_type_name,
                        edge_type.data,
                        decl.sentence.serialize()
                    ));
                }
            },
            None => {
                return Err(format!(
                    "Edge type {} not found in type mapping for declaration {:?}",
                    edge_type_name,
                    decl.sentence.serialize()
                ));
            }
        };

        Ok(())
    }
}
