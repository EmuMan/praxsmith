use std::collections::HashMap;

use crate::{
    definitions::{PraxsmthConstant, Serialize, types::*, world::*},
    store::{Handle, Store},
    types::TypeMapping,
};

pub enum Direction {
    Forward,
    Backward,
}

pub struct Trait {
    pub _type: TraitType,
    pub fields: HashMap<String, PraxsmthConstant>,
    pub agent_name: String,
}

pub struct Directional {
    pub _type: DirectionalType,
    pub fields: HashMap<String, PraxsmthConstant>,
    pub forward_agent_name: String,
    pub backward_agent_name: String,
}

pub struct Reciprocal {
    pub _type: ReciprocalType,
    pub fields: HashMap<String, PraxsmthConstant>,
    pub agents: (String, String), // (agent1, agent2) in some arbitrary order
}

pub struct Evaluation {
    pub _type: EvaluationType,
    pub fields: HashMap<String, PraxsmthConstant>,
    pub from_agent_name: String,
    pub to_agent_name: String,
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
