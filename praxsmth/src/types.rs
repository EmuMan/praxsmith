use std::collections::HashMap;

use crate::definitions::types::*;

pub struct TypeMapping {
    types: HashMap<String, PraxsmthType>,
    // I originally included a complement set here to track backwards edges better,
    // but I realized that those will be added as individual types anyways, so
    // they go through all the same validation as forward types eventually. Everything
    // is contained within this code, so there's really no chance of misaligned types.
}

impl TypeMapping {
    pub fn new() -> Self {
        TypeMapping {
            types: HashMap::new(),
        }
    }

    pub fn get_type(&self, name: &str) -> Option<&PraxsmthType> {
        self.types.get(name)
    }

    pub fn validate_new_name(&self, name: &str) -> Result<(), String> {
        if let Some(existing) = self.get_type(name) {
            Err(format!(
                "Some type with name {} already exists: {:?}",
                name, existing
            ))
        } else {
            Ok(())
        }
    }

    pub fn add_types(&mut self, types: Vec<PraxsmthType>) -> Result<(), String> {
        for t in types {
            self.add_type(t)?;
        }
        Ok(())
    }

    pub fn add_type(&mut self, t: PraxsmthType) -> Result<(), String> {
        self.validate_new_name(&t.name)?;
        self.types.insert(t.name.clone(), t);
        Ok(())
    }
}
