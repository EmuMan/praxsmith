use std::collections::HashMap;

use crate::{
    definitions::{PraxsmthConstant, PraxsmthField, Serialize, TypeFields, types::*, world::*},
    store::{Handle, Store},
    types::TypeMapping,
};

#[derive(Debug, Clone)]
pub enum Direction {
    Forward,
    Backward,
}

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

// =============================================================================
// WorldEdge trait
// =============================================================================

pub trait WorldEdge: Sized {
    type Query;

    fn matches(&self, query: &Self::Query) -> bool;
    fn fields(&self) -> &HashMap<String, PraxsmthConstant>;
    fn fields_mut(&mut self) -> &mut HashMap<String, PraxsmthConstant>;
    fn field_types(&self) -> &TypeFields;
    fn display_name() -> &'static str;
    fn iter_handles(agent: &Agent) -> impl Iterator<Item = Handle<Self>> + '_;
    fn remove_handle_at(agent: &mut Agent, pos: usize) -> Handle<Self>;
}

fn find_handle<E: WorldEdge>(
    agents: &HashMap<String, Agent>,
    store: &Store<E>,
    agent_name: &str,
    query: &E::Query,
) -> Result<Handle<E>, String> {
    let agent = agents
        .get(agent_name)
        .ok_or_else(|| format!("Agent {} does not exist", agent_name))?;
    E::iter_handles(agent)
        .find(|&h| {
            store
                .get(h)
                .unwrap_or_else(|| {
                    panic!(
                        "{} handle list for agent {} contains an invalid handle",
                        E::display_name(),
                        agent_name
                    )
                })
                .matches(query)
        })
        .ok_or_else(|| format!("Agent {} has no matching {}", agent_name, E::display_name()))
}

fn remove_edge<E: WorldEdge>(
    agents: &mut HashMap<String, Agent>,
    store: &mut Store<E>,
    agent_name: &str,
    query: &E::Query,
) -> Result<(), String> {
    let pos = {
        let agent = agents
            .get(agent_name)
            .ok_or_else(|| format!("Agent {} does not exist", agent_name))?;
        E::iter_handles(agent)
            .position(|h| {
                store
                    .get(h)
                    .unwrap_or_else(|| {
                        panic!(
                            "{} handle list for agent {} contains an invalid handle",
                            E::display_name(),
                            agent_name
                        )
                    })
                    .matches(query)
            })
            .ok_or_else(|| format!("Agent {} has no matching {}", agent_name, E::display_name()))?
    };
    let handle = E::remove_handle_at(
        agents
            .get_mut(agent_name)
            .expect("agent existence was already verified"),
        pos,
    );
    store.remove(handle).map_err(|e| e.to_string())
}

fn update_edge<E: WorldEdge>(
    agents: &HashMap<String, Agent>,
    store: &mut Store<E>,
    agent_name: &str,
    query: &E::Query,
    fields: HashMap<String, PraxsmthConstant>,
) -> Result<(), String> {
    let handle = find_handle(agents, store, agent_name, query)?;
    let edge = store
        .get_mut(handle)
        .expect("handle was just validated by find_handle");
    verify_fields(&fields, edge.field_types(), false)?;
    edge.fields_mut().extend(fields);
    Ok(())
}

// =============================================================================
// Edge types and query types
// =============================================================================

#[derive(Debug, Clone)]
pub struct Trait {
    pub _type: TraitType,
    pub fields: HashMap<String, PraxsmthConstant>,
    pub agent_name: String,
}

impl Trait {
    pub fn new(
        _type: TraitType,
        fields: HashMap<String, PraxsmthConstant>,
        agent_name: String,
    ) -> Result<Self, String> {
        verify_fields(&fields, &_type.fields, true)?;
        Ok(Trait {
            _type,
            fields,
            agent_name,
        })
    }
}

pub struct Directional {
    pub _type: DirectionalType,
    pub fields: HashMap<String, PraxsmthConstant>,
    pub forward_agent_name: String,
    pub backward_agent_name: String,
}

pub struct DirectionalQuery {
    pub edge_name: String,
    pub forward: String,
    pub backward: String,
}

pub struct Reciprocal {
    pub _type: ReciprocalType,
    pub fields: HashMap<String, PraxsmthConstant>,
    pub agents: (String, String),
}

pub struct ReciprocalQuery {
    pub name: String,
    pub agent_a: String,
    pub agent_b: String,
}

pub struct Evaluation {
    pub _type: EvaluationType,
    pub fields: HashMap<String, PraxsmthConstant>,
    pub from_agent_name: String,
    pub to_agent_name: String,
}

pub struct EvaluationQuery {
    pub edge_name: String,
    pub from: String,
    pub to: String,
}

pub struct Emotion {
    pub _type: EmotionType,
    pub fields: HashMap<String, PraxsmthConstant>,
    pub agent_name: String,
}

pub struct Practice {
    pub _type: PracticeType,
    pub fields: HashMap<String, PraxsmthConstant>,
    pub agent_names: Vec<String>,
}

pub struct PracticeQuery {
    pub name: String,
    pub agents: Vec<String>,
}

// =============================================================================
// WorldEdge implementations
// =============================================================================

impl WorldEdge for Trait {
    type Query = String;

    fn matches(&self, query: &String) -> bool {
        self._type.name == *query
    }
    fn fields(&self) -> &HashMap<String, PraxsmthConstant> {
        &self.fields
    }
    fn fields_mut(&mut self) -> &mut HashMap<String, PraxsmthConstant> {
        &mut self.fields
    }
    fn field_types(&self) -> &TypeFields {
        &self._type.fields
    }
    fn display_name() -> &'static str {
        "trait"
    }
    fn iter_handles(agent: &Agent) -> impl Iterator<Item = Handle<Self>> + '_ {
        agent.trait_handles.iter().copied()
    }
    fn remove_handle_at(agent: &mut Agent, pos: usize) -> Handle<Self> {
        agent.trait_handles.remove(pos)
    }
}

impl WorldEdge for Directional {
    type Query = DirectionalQuery;

    fn matches(&self, query: &DirectionalQuery) -> bool {
        (self._type.forward_name == query.edge_name
            || self._type.backward_name == query.edge_name)
            && self.forward_agent_name == query.forward
            && self.backward_agent_name == query.backward
    }
    fn fields(&self) -> &HashMap<String, PraxsmthConstant> {
        &self.fields
    }
    fn fields_mut(&mut self) -> &mut HashMap<String, PraxsmthConstant> {
        &mut self.fields
    }
    fn field_types(&self) -> &TypeFields {
        &self._type.fields
    }
    fn display_name() -> &'static str {
        "directional"
    }
    fn iter_handles(agent: &Agent) -> impl Iterator<Item = Handle<Self>> + '_ {
        agent.directional_handles.iter().map(|(h, _)| *h)
    }
    fn remove_handle_at(agent: &mut Agent, pos: usize) -> Handle<Self> {
        agent.directional_handles.remove(pos).0
    }
}

impl WorldEdge for Reciprocal {
    type Query = ReciprocalQuery;

    fn matches(&self, query: &ReciprocalQuery) -> bool {
        self._type.name == query.name
            && ((self.agents.0 == query.agent_a && self.agents.1 == query.agent_b)
                || (self.agents.0 == query.agent_b && self.agents.1 == query.agent_a))
    }
    fn fields(&self) -> &HashMap<String, PraxsmthConstant> {
        &self.fields
    }
    fn fields_mut(&mut self) -> &mut HashMap<String, PraxsmthConstant> {
        &mut self.fields
    }
    fn field_types(&self) -> &TypeFields {
        &self._type.fields
    }
    fn display_name() -> &'static str {
        "reciprocal"
    }
    fn iter_handles(agent: &Agent) -> impl Iterator<Item = Handle<Self>> + '_ {
        agent.reciprocal_handles.iter().copied()
    }
    fn remove_handle_at(agent: &mut Agent, pos: usize) -> Handle<Self> {
        agent.reciprocal_handles.remove(pos)
    }
}

impl WorldEdge for Evaluation {
    type Query = EvaluationQuery;

    fn matches(&self, query: &EvaluationQuery) -> bool {
        (self._type.forward_name == query.edge_name
            || self._type.backward_name == query.edge_name)
            && self.from_agent_name == query.from
            && self.to_agent_name == query.to
    }
    fn fields(&self) -> &HashMap<String, PraxsmthConstant> {
        &self.fields
    }
    fn fields_mut(&mut self) -> &mut HashMap<String, PraxsmthConstant> {
        &mut self.fields
    }
    fn field_types(&self) -> &TypeFields {
        &self._type.fields
    }
    fn display_name() -> &'static str {
        "evaluation"
    }
    fn iter_handles(agent: &Agent) -> impl Iterator<Item = Handle<Self>> + '_ {
        agent.evaluation_handles.iter().map(|(h, _)| *h)
    }
    fn remove_handle_at(agent: &mut Agent, pos: usize) -> Handle<Self> {
        agent.evaluation_handles.remove(pos).0
    }
}

impl WorldEdge for Emotion {
    type Query = String;

    fn matches(&self, query: &String) -> bool {
        self._type.name == *query
    }
    fn fields(&self) -> &HashMap<String, PraxsmthConstant> {
        &self.fields
    }
    fn fields_mut(&mut self) -> &mut HashMap<String, PraxsmthConstant> {
        &mut self.fields
    }
    fn field_types(&self) -> &TypeFields {
        &self._type.fields
    }
    fn display_name() -> &'static str {
        "emotion"
    }
    fn iter_handles(agent: &Agent) -> impl Iterator<Item = Handle<Self>> + '_ {
        agent.emotion_handle.iter().copied()
    }
    fn remove_handle_at(agent: &mut Agent, pos: usize) -> Handle<Self> {
        debug_assert_eq!(pos, 0, "emotion has at most one handle");
        agent
            .emotion_handle
            .take()
            .expect("emotion handle was present when position was found")
    }
}

impl WorldEdge for Practice {
    type Query = PracticeQuery;

    fn matches(&self, query: &PracticeQuery) -> bool {
        self._type.name == query.name && self.agent_names == query.agents
    }
    fn fields(&self) -> &HashMap<String, PraxsmthConstant> {
        &self.fields
    }
    fn fields_mut(&mut self) -> &mut HashMap<String, PraxsmthConstant> {
        &mut self.fields
    }
    fn field_types(&self) -> &TypeFields {
        &self._type.fields
    }
    fn display_name() -> &'static str {
        "practice"
    }
    fn iter_handles(agent: &Agent) -> impl Iterator<Item = Handle<Self>> + '_ {
        agent.practice_handles.iter().copied()
    }
    fn remove_handle_at(agent: &mut Agent, pos: usize) -> Handle<Self> {
        agent.practice_handles.remove(pos)
    }
}

// =============================================================================
// Agent and World
// =============================================================================

pub struct Agent {
    pub info: AgentInfo,
    pub trait_handles: Vec<Handle<Trait>>,
    pub directional_handles: Vec<(Handle<Directional>, Direction)>,
    pub reciprocal_handles: Vec<Handle<Reciprocal>>,
    pub evaluation_handles: Vec<(Handle<Evaluation>, Direction)>,
    pub emotion_handle: Option<Handle<Emotion>>,
    pub practice_handles: Vec<Handle<Practice>>,
}

impl Agent {
    pub fn new(info: AgentInfo) -> Self {
        Agent {
            info,
            trait_handles: Vec::new(),
            directional_handles: Vec::new(),
            reciprocal_handles: Vec::new(),
            evaluation_handles: Vec::new(),
            emotion_handle: None,
            practice_handles: Vec::new(),
        }
    }
}

pub struct World {
    pub agents: HashMap<String, Agent>,
    pub trait_store: Store<Trait>,
    pub directional_store: Store<Directional>,
    pub reciprocal_store: Store<Reciprocal>,
    pub evaluation_store: Store<Evaluation>,
    pub emotion_store: Store<Emotion>,
    pub practice_store: Store<Practice>,
    pub type_mapping: TypeMapping,
}

impl World {
    pub fn new() -> Self {
        World {
            agents: HashMap::new(),
            trait_store: Store::new(),
            directional_store: Store::new(),
            reciprocal_store: Store::new(),
            evaluation_store: Store::new(),
            emotion_store: Store::new(),
            practice_store: Store::new(),
            type_mapping: TypeMapping::new(),
        }
    }

    pub fn add_agent(&mut self, agent: AgentInfo) -> Result<(), String> {
        if self.agents.contains_key(&agent.name) {
            Err(format!("Agent {} already exists", agent.name))
        } else {
            self.agents.insert(agent.name.clone(), Agent::new(agent));
            Ok(())
        }
    }

    pub fn get_trait(&self, agent_name: &str, trait_name: &str) -> Option<&Trait> {
        let handle =
            find_handle(&self.agents, &self.trait_store, agent_name, &trait_name.to_string())
                .ok()?;
        self.trait_store.get(handle)
    }

    pub fn add_trait(
        &mut self,
        trait_def: TraitType,
        fields: HashMap<String, PraxsmthConstant>,
        agent_name: String,
    ) -> Result<(), String> {
        if find_handle(&self.agents, &self.trait_store, &agent_name, &trait_def.name).is_ok() {
            remove_edge(
                &mut self.agents,
                &mut self.trait_store,
                &agent_name,
                &trait_def.name,
            )?;
        }
        let agent = self
            .agents
            .get_mut(&agent_name)
            .ok_or_else(|| format!("Agent {} does not exist", agent_name))?;
        let new_trait = Trait::new(trait_def, fields, agent_name.clone())?;
        let handle = self.trait_store.add(new_trait);
        agent.trait_handles.push(handle);
        Ok(())
    }

    pub fn update_trait(
        &mut self,
        agent_name: &str,
        trait_name: &str,
        fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        update_edge(
            &self.agents,
            &mut self.trait_store,
            agent_name,
            &trait_name.to_string(),
            fields,
        )
    }

    pub fn remove_trait(&mut self, agent_name: &str, trait_name: &str) -> Result<(), String> {
        remove_edge(
            &mut self.agents,
            &mut self.trait_store,
            agent_name,
            &trait_name.to_string(),
        )
    }

    pub fn get_directional(&self, agent_name: &str, query: &DirectionalQuery) -> Option<&Directional> {
        let handle = find_handle(&self.agents, &self.directional_store, agent_name, query).ok()?;
        self.directional_store.get(handle)
    }

    pub fn update_directional(
        &mut self,
        agent_name: &str,
        query: &DirectionalQuery,
        fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        update_edge(&self.agents, &mut self.directional_store, agent_name, query, fields)
    }

    pub fn remove_directional(
        &mut self,
        agent_name: &str,
        query: &DirectionalQuery,
    ) -> Result<(), String> {
        remove_edge(&mut self.agents, &mut self.directional_store, agent_name, query)
    }

    pub fn get_reciprocal(&self, agent_name: &str, query: &ReciprocalQuery) -> Option<&Reciprocal> {
        let handle = find_handle(&self.agents, &self.reciprocal_store, agent_name, query).ok()?;
        self.reciprocal_store.get(handle)
    }

    pub fn update_reciprocal(
        &mut self,
        agent_name: &str,
        query: &ReciprocalQuery,
        fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        update_edge(&self.agents, &mut self.reciprocal_store, agent_name, query, fields)
    }

    pub fn remove_reciprocal(
        &mut self,
        agent_name: &str,
        query: &ReciprocalQuery,
    ) -> Result<(), String> {
        remove_edge(&mut self.agents, &mut self.reciprocal_store, agent_name, query)
    }

    pub fn get_evaluation(&self, agent_name: &str, query: &EvaluationQuery) -> Option<&Evaluation> {
        let handle = find_handle(&self.agents, &self.evaluation_store, agent_name, query).ok()?;
        self.evaluation_store.get(handle)
    }

    pub fn update_evaluation(
        &mut self,
        agent_name: &str,
        query: &EvaluationQuery,
        fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        update_edge(&self.agents, &mut self.evaluation_store, agent_name, query, fields)
    }

    pub fn remove_evaluation(
        &mut self,
        agent_name: &str,
        query: &EvaluationQuery,
    ) -> Result<(), String> {
        remove_edge(&mut self.agents, &mut self.evaluation_store, agent_name, query)
    }

    pub fn get_emotion(&self, agent_name: &str, emotion_name: &str) -> Option<&Emotion> {
        let handle = find_handle(
            &self.agents,
            &self.emotion_store,
            agent_name,
            &emotion_name.to_string(),
        )
        .ok()?;
        self.emotion_store.get(handle)
    }

    pub fn update_emotion(
        &mut self,
        agent_name: &str,
        emotion_name: &str,
        fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        update_edge(
            &self.agents,
            &mut self.emotion_store,
            agent_name,
            &emotion_name.to_string(),
            fields,
        )
    }

    pub fn remove_emotion(&mut self, agent_name: &str, emotion_name: &str) -> Result<(), String> {
        remove_edge(
            &mut self.agents,
            &mut self.emotion_store,
            agent_name,
            &emotion_name.to_string(),
        )
    }

    pub fn get_practice(&self, agent_name: &str, query: &PracticeQuery) -> Option<&Practice> {
        let handle = find_handle(&self.agents, &self.practice_store, agent_name, query).ok()?;
        self.practice_store.get(handle)
    }

    pub fn update_practice(
        &mut self,
        agent_name: &str,
        query: &PracticeQuery,
        fields: HashMap<String, PraxsmthConstant>,
    ) -> Result<(), String> {
        update_edge(&self.agents, &mut self.practice_store, agent_name, query, fields)
    }

    pub fn remove_practice(
        &mut self,
        agent_name: &str,
        query: &PracticeQuery,
    ) -> Result<(), String> {
        remove_edge(&mut self.agents, &mut self.practice_store, agent_name, query)
    }

    pub fn process_declaration(&mut self, decl: &Declaration) -> Result<(), String> {
        if decl.sentence.len() < 3 {
            return Err(format!(
                "Declaration sentence must have at least 3 parts: {:?}",
                decl.sentence.serialize()
            ));
        }

        unimplemented!();
    }
}
