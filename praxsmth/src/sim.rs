use std::collections::HashMap;

use crate::definitions::{types::*, world::*};

pub struct TypeMapping {
    pub traits: HashMap<String, Trait>,
    pub directionals: HashMap<String, Directional>,
    pub directional_reversals: HashMap<String, String>, // forward_name -> backward_name
    pub reciprocals: HashMap<String, Reciprocal>,
    pub evaluations: HashMap<String, Evaluation>,
    pub evaluation_reversals: HashMap<String, String>, // forward_name -> backward_name
    pub emotions: HashMap<String, Emotion>,
    pub practices: HashMap<String, Practice>,
}

pub struct World {
    agents: HashMap<String, Agent>,
    type_mapping: TypeMapping,
}

impl World {
    pub fn new() -> Self {
        World {
            agents: HashMap::new(),
            type_mapping: TypeMapping {
                traits: HashMap::new(),
                directionals: HashMap::new(),
                directional_reversals: HashMap::new(),
                reciprocals: HashMap::new(),
                evaluations: HashMap::new(),
                evaluation_reversals: HashMap::new(),
                emotions: HashMap::new(),
                practices: HashMap::new(),
            },
        }
    }

    fn name_exists(&self, name: &str) -> bool {
        self.type_mapping.traits.contains_key(name)
            || self.type_mapping.directionals.contains_key(name)
            || self.type_mapping.directional_reversals.contains_key(name)
            || self.type_mapping.reciprocals.contains_key(name)
            || self.type_mapping.evaluations.contains_key(name)
            || self.type_mapping.evaluation_reversals.contains_key(name)
            || self.type_mapping.emotions.contains_key(name)
            || self.type_mapping.practices.contains_key(name)
    }

    /// Adds a trait to the world.
    /// Returns an error if a type with the same name already exists in the world.
    pub fn add_trait(&mut self, trait_def: Trait) -> Result<(), String> {
        if self.name_exists(&trait_def.name) {
            Err(format!(
                "Some type with name {} already exists",
                trait_def.name
            ))
        } else {
            self.type_mapping
                .traits
                .insert(trait_def.name.clone(), trait_def);
            Ok(())
        }
    }

    /// Adds a directional type to the world. Ensures that both forward and backward names are unique.
    /// Returns an error if a type with the same forward or backward name already exists in the world.
    pub fn add_directional(&mut self, dir_def: Directional) -> Result<(), String> {
        if self.name_exists(&dir_def.forward_name) {
            Err(format!(
                "Some type with name {} already exists",
                dir_def.forward_name
            ))
        } else if self.name_exists(&dir_def.backward_name) {
            Err(format!(
                "Some type with name {} already exists",
                dir_def.backward_name
            ))
        } else if dir_def.forward_name == dir_def.backward_name {
            Err(format!(
                "Directional type {} cannot have the same forward and backward name. Use a reciprocal type instead.",
                dir_def.forward_name
            ))
        } else {
            self.type_mapping
                .directional_reversals
                .insert(dir_def.backward_name.clone(), dir_def.forward_name.clone());
            self.type_mapping
                .directionals
                .insert(dir_def.forward_name.clone(), dir_def);
            Ok(())
        }
    }

    /// Adds a reciprocal type to the world. Ensures that the name is unique.
    /// Returns an error if a type with the same name already exists in the world.
    pub fn add_reciprocal(&mut self, rec_def: Reciprocal) -> Result<(), String> {
        if self.name_exists(&rec_def.name) {
            Err(format!(
                "Some type with name {} already exists",
                rec_def.name
            ))
        } else {
            self.type_mapping
                .reciprocals
                .insert(rec_def.name.clone(), rec_def);
            Ok(())
        }
    }

    /// Adds an evaluation type to the world. Ensures that both forward and backward names are unique.
    /// Returns an error if a type with the same forward or backward name already exists in the world.
    pub fn add_evaluation(&mut self, eval_def: Evaluation) -> Result<(), String> {
        if self.name_exists(&eval_def.forward_name) {
            Err(format!(
                "Some type with name {} already exists",
                eval_def.forward_name
            ))
        } else if self.name_exists(&eval_def.backward_name) {
            Err(format!(
                "Some type with name {} already exists",
                eval_def.backward_name
            ))
        } else if eval_def.forward_name == eval_def.backward_name {
            Err(format!(
                "Evaluation type {} cannot have the same forward and backward name.",
                eval_def.forward_name
            ))
        } else {
            self.type_mapping.evaluation_reversals.insert(
                eval_def.backward_name.clone(),
                eval_def.forward_name.clone(),
            );
            self.type_mapping
                .evaluations
                .insert(eval_def.forward_name.clone(), eval_def);
            Ok(())
        }
    }

    /// Adds an emotion type to the world. Ensures that the name is unique.
    /// Returns an error if a type with the same name already exists in the world.
    pub fn add_emotion(&mut self, em_def: Emotion) -> Result<(), String> {
        if self.name_exists(&em_def.name) {
            Err(format!(
                "Some type with name {} already exists",
                em_def.name
            ))
        } else {
            self.type_mapping
                .emotions
                .insert(em_def.name.clone(), em_def);
            Ok(())
        }
    }

    /// Adds a practice type to the world. Ensures that the name is unique.
    /// Returns an error if a type with the same name already exists in the world.
    pub fn add_practice(&mut self, prac_def: Practice) -> Result<(), String> {
        if self.name_exists(&prac_def.name) {
            Err(format!(
                "Some type with name {} already exists",
                prac_def.name
            ))
        } else {
            self.type_mapping
                .practices
                .insert(prac_def.name.clone(), prac_def);
            Ok(())
        }
    }

    pub fn add_type(&mut self, t: PraxsmthType) -> Result<(), String> {
        match t {
            PraxsmthType::Trait(trait_def) => self.add_trait(trait_def),
            PraxsmthType::Directional(dir_def) => self.add_directional(dir_def),
            PraxsmthType::Reciprocal(rec_def) => self.add_reciprocal(rec_def),
            PraxsmthType::Evaluation(eval_def) => self.add_evaluation(eval_def),
            PraxsmthType::Emotion(em_def) => self.add_emotion(em_def),
            PraxsmthType::Practice(prac_def) => self.add_practice(prac_def),
        }
    }

    pub fn add_agent(&mut self, agent: Agent) -> Result<(), String> {
        if self.agents.contains_key(&agent.name) {
            Err(format!("Agent {} already exists", agent.name))
        } else {
            self.agents.insert(agent.name.clone(), agent);
            Ok(())
        }
    }

    pub fn process_declaration(&mut self, decl: Declaration) -> Result<(), String> {
        unimplemented!()
    }
}
