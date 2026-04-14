use std::collections::HashMap;

use crate::definitions::types::*;

pub struct TypeMapping {
    pub traits: HashMap<String, TraitType>,
    pub directionals: HashMap<String, DirectionalType>,
    pub directional_reversals: HashMap<String, String>, // forward_name -> backward_name
    pub reciprocals: HashMap<String, ReciprocalType>,
    pub evaluations: HashMap<String, EvaluationType>,
    pub evaluation_reversals: HashMap<String, String>, // forward_name -> backward_name
    pub emotions: HashMap<String, EmotionType>,
    pub practices: HashMap<String, PracticeType>,
}

impl TypeMapping {
    pub fn new() -> Self {
        TypeMapping {
            traits: HashMap::new(),
            directionals: HashMap::new(),
            directional_reversals: HashMap::new(),
            reciprocals: HashMap::new(),
            evaluations: HashMap::new(),
            evaluation_reversals: HashMap::new(),
            emotions: HashMap::new(),
            practices: HashMap::new(),
        }
    }

    fn name_exists(&self, name: &str) -> bool {
        self.traits.contains_key(name)
            || self.directionals.contains_key(name)
            || self.directional_reversals.contains_key(name)
            || self.reciprocals.contains_key(name)
            || self.evaluations.contains_key(name)
            || self.evaluation_reversals.contains_key(name)
            || self.emotions.contains_key(name)
            || self.practices.contains_key(name)
    }

    /// Adds a trait to the mapping.
    /// Returns an error if a type with the same name already exists in the mapping.
    pub fn add_trait(&mut self, trait_def: TraitType) -> Result<(), String> {
        if self.name_exists(&trait_def.name) {
            Err(format!(
                "Some type with name {} already exists",
                trait_def.name
            ))
        } else {
            self.traits.insert(trait_def.name.clone(), trait_def);
            Ok(())
        }
    }

    /// Adds a directional type to the mapping. Ensures that both forward and backward names are unique.
    /// Returns an error if a type with the same forward or backward name already exists in the mapping.
    pub fn add_directional(&mut self, dir_def: DirectionalType) -> Result<(), String> {
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
            self.directional_reversals
                .insert(dir_def.backward_name.clone(), dir_def.forward_name.clone());
            self.directionals
                .insert(dir_def.forward_name.clone(), dir_def);
            Ok(())
        }
    }

    /// Adds a reciprocal type to the mapping. Ensures that the name is unique.
    /// Returns an error if a type with the same name already exists in the mapping.
    pub fn add_reciprocal(&mut self, rec_def: ReciprocalType) -> Result<(), String> {
        if self.name_exists(&rec_def.name) {
            Err(format!(
                "Some type with name {} already exists",
                rec_def.name
            ))
        } else {
            self.reciprocals.insert(rec_def.name.clone(), rec_def);
            Ok(())
        }
    }

    /// Adds an evaluation type to the mapping. Ensures that both forward and backward names are unique.
    /// Returns an error if a type with the same forward or backward name already exists in the mapping.
    pub fn add_evaluation(&mut self, eval_def: EvaluationType) -> Result<(), String> {
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
            self.evaluation_reversals.insert(
                eval_def.backward_name.clone(),
                eval_def.forward_name.clone(),
            );
            self.evaluations
                .insert(eval_def.forward_name.clone(), eval_def);
            Ok(())
        }
    }

    /// Adds an emotion type to the mapping. Ensures that the name is unique.
    /// Returns an error if a type with the same name already exists in the mapping.
    pub fn add_emotion(&mut self, em_def: EmotionType) -> Result<(), String> {
        if self.name_exists(&em_def.name) {
            Err(format!(
                "Some type with name {} already exists",
                em_def.name
            ))
        } else {
            self.emotions.insert(em_def.name.clone(), em_def);
            Ok(())
        }
    }

    /// Adds a practice type to the mapping. Ensures that the name is unique.
    /// Returns an error if a type with the same name already exists in the mapping.
    pub fn add_practice(&mut self, prac_def: PracticeType) -> Result<(), String> {
        if self.name_exists(&prac_def.name) {
            Err(format!(
                "Some type with name {} already exists",
                prac_def.name
            ))
        } else {
            self.practices.insert(prac_def.name.clone(), prac_def);
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
}
