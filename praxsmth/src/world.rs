use std::collections::HashMap;

use crate::{
    definitions::{PraxsmthConstant, PraxsmthField, Serialize, TypeFields, world::*},
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
pub struct WorldEdge {
    type_name: String,
    from: String,
    fields: HashMap<String, PraxsmthConstant>,
    data: EdgeData,
}

impl WorldEdge {
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
pub enum EdgeData {
    Trait,
    DirectionalForward {
        to: String,
        complement_handle: EdgeHandle,
    },
    DirectionalBackward {
        to: String,
        complement_handle: EdgeHandle,
    },
    Reciprocal {
        to: String,
        complement_handle: EdgeHandle,
    },
    EvaluationForward {
        to: String,
        complement_handle: EdgeHandle,
        reason: String,
    },
    EvaluationBackward {
        to: String,
        complement_handle: EdgeHandle,
        reason: String,
    },
    Emotion,
    Practice {
        participants: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct EdgeHandle {
    index: u32,
    generation: u32,
}

struct EdgeStoreSlot {
    value: Option<WorldEdge>,
    generation: u32,
}

pub struct EdgeStore {
    slots: Vec<EdgeStoreSlot>,
    open_indices: Vec<usize>,
}

impl EdgeStore {
    pub fn new() -> Self {
        EdgeStore {
            slots: Vec::new(),
            open_indices: Vec::new(),
        }
    }

    pub fn peek_next_two_handles(&self) -> (EdgeHandle, EdgeHandle) {
        if self.open_indices.is_empty() {
            let new_index = self.slots.len();
            (
                EdgeHandle {
                    index: new_index as u32,
                    generation: 0,
                },
                EdgeHandle {
                    index: (new_index + 1) as u32,
                    generation: 0,
                },
            )
        } else if self.open_indices.len() == 1 {
            let slot_index = self.open_indices[0];
            (
                EdgeHandle {
                    index: slot_index as u32,
                    generation: self.slots[slot_index].generation,
                },
                EdgeHandle {
                    index: self.slots.len() as u32,
                    generation: 0,
                },
            )
        } else {
            let slot_index1 = self.open_indices[self.open_indices.len() - 1];
            let slot_index2 = self.open_indices[self.open_indices.len() - 2];
            (
                EdgeHandle {
                    index: slot_index1 as u32,
                    generation: self.slots[slot_index1].generation,
                },
                EdgeHandle {
                    index: slot_index2 as u32,
                    generation: self.slots[slot_index2].generation,
                },
            )
        }
    }

    pub fn add(&mut self, edge: WorldEdge) -> EdgeHandle {
        if let Some(slot_index) = self.open_indices.pop() {
            let slot = &mut self.slots[slot_index];
            slot.value = Some(edge);
            EdgeHandle {
                index: slot_index as u32,
                generation: slot.generation,
            }
        } else {
            let new_index = self.slots.len();
            self.slots.push(EdgeStoreSlot {
                value: Some(edge),
                generation: 0,
            });
            EdgeHandle {
                index: new_index as u32,
                generation: 0,
            }
        }
    }

    pub fn get(&self, handle: EdgeHandle) -> Option<&WorldEdge> {
        self.slots.get(handle.index as usize).and_then(|slot| {
            if slot.generation == handle.generation {
                slot.value.as_ref()
            } else {
                None
            }
        })
    }

    pub fn get_mut(&mut self, handle: EdgeHandle) -> Option<&mut WorldEdge> {
        self.slots.get_mut(handle.index as usize).and_then(|slot| {
            if slot.generation == handle.generation {
                slot.value.as_mut()
            } else {
                None
            }
        })
    }

    pub fn remove(&mut self, handle: EdgeHandle) -> Result<(), String> {
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
    pub edges: Vec<EdgeHandle>,
}

impl Agent {
    pub fn new(_info: AgentInfo) -> Self {
        // TODO: better agent construction
        Agent { edges: Vec::new() }
    }
}

pub struct World {
    pub agents: HashMap<String, Agent>,
    pub type_mapping: TypeMapping,
    pub edge_store: EdgeStore,
}

impl World {
    pub fn new() -> Self {
        World {
            agents: HashMap::new(),
            type_mapping: TypeMapping::new(),
            edge_store: EdgeStore::new(),
        }
    }

    pub fn iter_edges(&self) -> impl Iterator<Item = (EdgeHandle, &WorldEdge)> {
        self.edge_store
            .slots
            .iter()
            .enumerate()
            .filter_map(|(index, slot)| {
                slot.value.as_ref().map(|edge| {
                    (
                        EdgeHandle {
                            index: index as u32,
                            generation: slot.generation,
                        },
                        edge,
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

    pub fn get_edge(&self, handle: EdgeHandle) -> Option<&WorldEdge> {
        self.edge_store.get(handle)
    }

    pub fn add_edge(&mut self, edge: WorldEdge) -> Result<EdgeHandle, String> {
        if let Some(agent) = self.agents.get_mut(&edge.from) {
            let handle = self.edge_store.add(edge);
            agent.edges.push(handle.clone());
            Ok(handle)
        } else {
            Err(format!("Agent with name {} not found", edge.from))
        }
    }

    pub fn remove_edge(&mut self, handle: EdgeHandle, propogate: bool) -> Result<(), String> {
        match self.edge_store.get(handle.clone()) {
            Some(edge) => {
                if let Some(agent) = self.agents.get_mut(&edge.from) {
                    agent.edges.retain(|h| h != &handle);
                }
                if propogate {
                    match &edge.data {
                        EdgeData::DirectionalForward {
                            complement_handle, ..
                        }
                        | EdgeData::DirectionalBackward {
                            complement_handle, ..
                        }
                        | EdgeData::Reciprocal {
                            complement_handle, ..
                        }
                        | EdgeData::EvaluationForward {
                            complement_handle, ..
                        }
                        | EdgeData::EvaluationBackward {
                            complement_handle, ..
                        } => {
                            self.remove_edge(complement_handle.clone(), false)?;
                        }
                        _ => {}
                    }
                }
            }
            None => {
                return Err(format!("Edge handle {:?} not found", handle));
            }
        }
        self.edge_store.remove(handle)
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

    /// Adds two complementary edges atomically, using peeked handles to wire up
    /// each edge's complement before either is stored. `make_data(h1, h2)` receives
    /// the handles in insertion order and must return `(data_for_edge1, data_for_edge2)`.
    fn add_paired_edges(
        &mut self,
        from: &str,
        to: &str,
        fields: HashMap<String, PraxsmthConstant>,
        type_name: &str,
        make_data: impl FnOnce(EdgeHandle, EdgeHandle) -> (EdgeData, EdgeData),
    ) -> Result<(EdgeHandle, EdgeHandle), String> {
        let (handle1, handle2) = self.edge_store.peek_next_two_handles();
        let (data1, data2) = make_data(handle1.clone(), handle2.clone());
        let edge1 = WorldEdge {
            type_name: type_name.to_string(),
            from: from.to_string(),
            fields: fields.clone(),
            data: data1,
        };
        let edge2 = WorldEdge {
            type_name: type_name.to_string(),
            from: to.to_string(),
            fields,
            data: data2,
        };
        let new_handle1 = self.add_edge(edge1)?;
        let new_handle2 = self.add_edge(edge2)?;
        if new_handle1 != handle1 || new_handle2 != handle2 {
            panic!(
                "Edge store peeked handles {:?} and {:?} but returned different handles {:?} and {:?} when adding edges",
                handle1, handle2, new_handle1, new_handle2
            );
        }
        Ok((new_handle1, new_handle2))
    }

    /// Updates an edge's fields after `validate_data` confirms the edge variant is correct.
    /// Used for edges that have no complement (Trait, Emotion).
    fn update_edge_simple(
        &mut self,
        handle: EdgeHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
        validate_data: impl FnOnce(&EdgeData) -> Result<(), String>,
    ) -> Result<(), String> {
        match self.edge_store.get_mut(handle.clone()) {
            Some(edge) => {
                validate_data(&edge.data)?;
                match self.type_mapping.get_type(&edge.type_name) {
                    Some(edge_type) => edge.update_fields(new_fields, &edge_type.fields),
                    None => Err(format!("Type {} not found in type mapping", edge.type_name)),
                }
            }
            None => Err(format!("Edge with handle {:?} not found", handle)),
        }
    }

    /// Updates an edge's fields and returns its complement handle. `extract_complement`
    /// validates the edge variant, optionally mutates extra fields (e.g. `reason`), and
    /// returns the complement handle. Used for edges that come in pairs.
    fn update_edge_with_complement(
        &mut self,
        handle: EdgeHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
        extract_complement: impl FnOnce(&mut EdgeData) -> Result<EdgeHandle, String>,
    ) -> Result<EdgeHandle, String> {
        match self.edge_store.get_mut(handle.clone()) {
            Some(edge) => {
                let complement_handle = extract_complement(&mut edge.data)?;
                match self.type_mapping.get_type(&edge.type_name) {
                    Some(edge_type) => {
                        edge.update_fields(new_fields, &edge_type.fields)?;
                        Ok(complement_handle)
                    }
                    None => Err(format!("Type {} not found in type mapping", edge.type_name)),
                }
            }
            None => Err(format!("Edge with handle {:?} not found", handle)),
        }
    }

    pub fn add_trait(
        &mut self,
        from: &str,
        fields: HashMap<String, PraxsmthConstant>,
        type_name: &str,
    ) -> Result<EdgeHandle, String> {
        self.validate_type_fields(type_name, &fields)?;
        self.add_edge(WorldEdge {
            type_name: type_name.to_string(),
            from: from.to_string(),
            fields,
            data: EdgeData::Trait,
        })
    }

    pub fn update_trait(
        &mut self,
        handle: EdgeHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        self.update_edge_simple(handle.clone(), new_fields, |data| match data {
            EdgeData::Trait => Ok(()),
            _ => Err(format!("Edge with handle {:?} is not a trait edge", handle)),
        })
    }

    pub fn add_directional(
        &mut self,
        from: &str,
        to: &str,
        fields: HashMap<String, PraxsmthConstant>,
        type_name: &str,
    ) -> Result<(EdgeHandle, EdgeHandle), String> {
        self.validate_type_fields(type_name, &fields)?;
        let to_owned = to.to_string();
        let from_owned = from.to_string();
        self.add_paired_edges(from, to, fields, type_name, |h1, h2| {
            (
                EdgeData::DirectionalForward {
                    to: to_owned,
                    complement_handle: h2,
                },
                EdgeData::DirectionalBackward {
                    to: from_owned,
                    complement_handle: h1,
                },
            )
        })
    }

    fn update_directional_nonpropagate(
        &mut self,
        handle: EdgeHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<EdgeHandle, String> {
        self.update_edge_with_complement(handle.clone(), new_fields, |data| match data {
            EdgeData::DirectionalForward {
                complement_handle, ..
            }
            | EdgeData::DirectionalBackward {
                complement_handle, ..
            } => Ok(complement_handle.clone()),
            _ => Err(format!(
                "Edge with handle {:?} is not a directional edge",
                handle
            )),
        })
    }

    pub fn update_directional(
        &mut self,
        handle: EdgeHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        let complement_handle = self.update_directional_nonpropagate(handle, new_fields.clone())?;
        self.update_directional_nonpropagate(complement_handle, new_fields)?;
        Ok(())
    }

    pub fn add_reciprocal(
        &mut self,
        from: &str,
        to: &str,
        fields: HashMap<String, PraxsmthConstant>,
        type_name: &str,
    ) -> Result<(EdgeHandle, EdgeHandle), String> {
        self.validate_type_fields(type_name, &fields)?;
        let to_owned = to.to_string();
        let from_owned = from.to_string();
        self.add_paired_edges(from, to, fields, type_name, |h1, h2| {
            (
                EdgeData::Reciprocal {
                    to: to_owned,
                    complement_handle: h2,
                },
                EdgeData::Reciprocal {
                    to: from_owned,
                    complement_handle: h1,
                },
            )
        })
    }

    fn update_reciprocal_nonpropagate(
        &mut self,
        handle: EdgeHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<EdgeHandle, String> {
        self.update_edge_with_complement(handle.clone(), new_fields, |data| match data {
            EdgeData::Reciprocal {
                complement_handle, ..
            } => Ok(complement_handle.clone()),
            _ => Err(format!(
                "Edge with handle {:?} is not a reciprocal edge",
                handle
            )),
        })
    }

    pub fn update_reciprocal(
        &mut self,
        handle: EdgeHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        let complement_handle = self.update_reciprocal_nonpropagate(handle, new_fields.clone())?;
        self.update_reciprocal_nonpropagate(complement_handle, new_fields)?;
        Ok(())
    }

    pub fn add_evaluation(
        &mut self,
        from: &str,
        to: &str,
        fields: HashMap<String, PraxsmthConstant>,
        type_name: &str,
        reason: &str,
    ) -> Result<(EdgeHandle, EdgeHandle), String> {
        self.validate_type_fields(type_name, &fields)?;
        let to_owned = to.to_string();
        let from_owned = from.to_string();
        let reason_owned = reason.to_string();
        self.add_paired_edges(from, to, fields, type_name, |h1, h2| {
            (
                EdgeData::EvaluationForward {
                    to: to_owned,
                    complement_handle: h2,
                    reason: reason_owned.clone(),
                },
                EdgeData::EvaluationBackward {
                    to: from_owned,
                    complement_handle: h1,
                    reason: reason_owned,
                },
            )
        })
    }

    fn update_evaluation_nonpropagate(
        &mut self,
        handle: EdgeHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
        new_reason: &str,
    ) -> Result<EdgeHandle, String> {
        let new_reason = new_reason.to_string();
        self.update_edge_with_complement(handle.clone(), new_fields, |data| match data {
            EdgeData::EvaluationForward {
                complement_handle,
                reason,
                ..
            }
            | EdgeData::EvaluationBackward {
                complement_handle,
                reason,
                ..
            } => {
                *reason = new_reason;
                Ok(complement_handle.clone())
            }
            _ => Err(format!(
                "Edge with handle {:?} is not an evaluation edge",
                handle
            )),
        })
    }

    pub fn update_evaluation(
        &mut self,
        handle: EdgeHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
        new_reason: &str,
    ) -> Result<(), String> {
        let complement_handle =
            self.update_evaluation_nonpropagate(handle, new_fields.clone(), new_reason)?;
        self.update_evaluation_nonpropagate(complement_handle, new_fields, new_reason)?;
        Ok(())
    }

    pub fn add_emotion(
        &mut self,
        from: &str,
        fields: HashMap<String, PraxsmthConstant>,
        type_name: &str,
    ) -> Result<EdgeHandle, String> {
        self.validate_type_fields(type_name, &fields)?;

        let edge = WorldEdge {
            type_name: type_name.to_string(),
            from: from.to_string(),
            fields,
            data: EdgeData::Emotion,
        };

        // Remove existing emotion edge if it exists
        if let Some(agent) = self.agents.get(from) {
            for edge_handle in &agent.edges {
                if let Some(existing_edge) = self.edge_store.get(edge_handle.clone()) {
                    if let EdgeData::Emotion = existing_edge.data {
                        self.remove_edge(edge_handle.clone(), false)?;
                        break;
                    }
                }
            }
        }

        self.add_edge(edge)
    }

    pub fn update_emotion(
        &mut self,
        handle: EdgeHandle,
        new_fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        self.update_edge_simple(handle.clone(), new_fields, |data| match data {
            EdgeData::Emotion => Ok(()),
            _ => Err(format!(
                "Edge with handle {:?} is not an emotion edge",
                handle
            )),
        })
    }

    // TODO: add similar functions for practices

    pub fn process_declaration(&mut self, decl: Declaration) -> Result<(), String> {
        // if decl.sentence.len() < 3 {
        //     return Err(format!(
        //         "Declaration sentence must have at least 3 parts: {:?}",
        //         decl.sentence.serialize()
        //     ));
        // }

        // match decl.sentence[1].as_str() {
        //     "is" => {
        //         // Trait declaration: "<agent>.is.<trait>"
        //         let agent_name = &decl.sentence[0];
        //         let trait_name = &decl.sentence[2];
        //         self.add_trait(decl.fields, agent_name.clone());
        //     }
        // }
        Ok(())
    }
}
