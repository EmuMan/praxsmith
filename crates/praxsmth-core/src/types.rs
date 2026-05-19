use std::collections::HashMap;

use anyhow::{Context, Result, bail};

use crate::definitions::types::*;

pub enum TypeMappingEntry {
    Type(PraxsmthType),
    Complement(String),
}

pub struct TypeMapping {
    types: HashMap<String, TypeMappingEntry>,
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

    pub fn from_types(types: Vec<PraxsmthType>) -> Result<Self> {
        let mut mapping = TypeMapping::new();
        mapping
            .add_types(types)
            .context("building type mapping from types list")?;
        Ok(mapping)
    }

    /// Gets the type with the given name, if it exists and is a primary type.
    /// Returns None if the name is not found or if it is a complement.
    pub fn get_type(&self, name: &str) -> Option<&PraxsmthType> {
        match self.types.get(name) {
            Some(TypeMappingEntry::Type(t)) => Some(t),
            Some(TypeMappingEntry::Complement(_)) => None,
            _ => None,
        }
    }

    /// Gets the type with the given name, if it exists. If the name is a
    /// complement, it follows the chain of complements until it finds a
    /// primary type or returns None if it doesn't find one.
    pub fn get_type_or_primary(&self, name: &str) -> Option<&PraxsmthType> {
        match self.types.get(name) {
            Some(TypeMappingEntry::Type(t)) => Some(t),
            Some(TypeMappingEntry::Complement(other)) => self.get_type_or_primary(other),
            _ => None,
        }
    }

    /// Gets the primary name for the given name, if it exists. If the name is
    /// a complement, it follows the chain of complements until it finds a
    /// primary type or returns None if it doesn't find one.
    ///
    /// Unlike `TypeMapping::get_complement_entry`, this will follow the chain
    /// of complements and return the primary name even if the input is the
    /// primary name itself.
    pub fn get_primary_name(&self, name: &str) -> Option<&str> {
        match self.types.get(name) {
            Some(TypeMappingEntry::Type(t)) => Some(&t.name),
            Some(TypeMappingEntry::Complement(other)) => self.get_primary_name(other),
            _ => None,
        }
    }

    /// Gets the referred name for the given complement, if it exists. If the
    /// name is a complement, it returns the name it refers to. If the name is
    /// a primary type, it returns None.
    ///
    /// Unlike `TypeMapping::get_primary_name`, this does not follow the chain
    /// of complements and will return `None` on primary types.
    pub fn get_complement_entry(&self, name: &str) -> Option<&str> {
        match self.types.get(name) {
            Some(TypeMappingEntry::Complement(other)) => Some(other),
            _ => None,
        }
    }

    pub fn validate_new_name(&self, name: &str) -> Result<()> {
        if let Some(existing) = self.get_type_or_primary(name) {
            bail!(
                "some type with name {} already exists: {:?}",
                name,
                existing
            );
        }
        Ok(())
    }

    pub fn add_types(&mut self, types: Vec<PraxsmthType>) -> Result<()> {
        for t in types {
            let name = t.name.clone();
            self.add_type(t)
                .with_context(|| format!("adding type {}", name))?;
        }
        Ok(())
    }

    pub fn add_type(&mut self, t: PraxsmthType) -> Result<()> {
        self.validate_new_name(&t.name)
            .with_context(|| format!("validating new type name {}", t.name))?;
        match &t.data {
            PraxsmthTypeData::Directional { complement, .. } => {
                self.validate_new_name(complement)
                    .with_context(|| format!("validating complement name {}", complement))?;
                self.types.insert(
                    complement.clone(),
                    TypeMappingEntry::Complement(t.name.clone()),
                );
            }
            PraxsmthTypeData::Evaluation { complement } => {
                self.validate_new_name(complement)
                    .with_context(|| format!("validating complement name {}", complement))?;
                self.types.insert(
                    complement.clone(),
                    TypeMappingEntry::Complement(t.name.clone()),
                );
            }
            _ => {}
        }
        self.types.insert(t.name.clone(), TypeMappingEntry::Type(t));
        Ok(())
    }
}
