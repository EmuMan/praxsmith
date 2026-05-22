use std::{collections::HashMap, fmt};

use anyhow::{Context, Result, bail};

use crate::{expressions::Expression, world::simulation::Effect};

#[derive(Debug, Clone)]
pub struct RelationType {
    pub name: String,
    pub fields: FieldTypes,
    pub data: RelationTypeData,
}

impl fmt::Display for RelationType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.data {
            RelationTypeData::Trait => {
                write!(f, "trait {} {{{}}}", self.name, self.fields.to_string())
            }
            RelationTypeData::Directional {
                complement,
                exclusive,
            } => write!(
                f,
                "{}directional {}/{} {{{}}}",
                if *exclusive { "exclusive " } else { "" },
                self.name,
                complement,
                self.fields.to_string()
            ),
            RelationTypeData::Reciprocal => {
                write!(
                    f,
                    "reciprocal {} {{{}}}",
                    self.name,
                    self.fields.to_string()
                )
            }
            RelationTypeData::Evaluation { complement } => {
                write!(
                    f,
                    "evaluation {}/{} {{{}}}",
                    self.name,
                    complement,
                    self.fields.to_string()
                )
            }
            RelationTypeData::Emotion => {
                write!(f, "emotion {} {{{}}}", self.name, self.fields.to_string())
            }
            RelationTypeData::Practice { params, .. } => {
                let params_str = params.join(", ");
                write!(
                    f,
                    "practice {}({}) {{{}}}",
                    self.name,
                    params_str,
                    self.fields.to_string()
                )
            }
        }
    }
}

#[derive(Debug, Clone)]
pub enum RelationTypeData {
    Trait,
    Directional {
        complement: String,
        exclusive: bool,
    },
    Reciprocal,
    Evaluation {
        complement: String,
    },
    Emotion,
    Practice {
        params: Vec<String>,
        actions: Vec<PracticeAction>,
    },
}

#[derive(Debug, Clone)]
pub struct PracticeAction {
    pub for_actor: String,
    pub name: String,
    pub conditions: Vec<Expression>,
    pub effects: Vec<Effect>,
}

#[derive(Debug, Clone)]
pub enum FieldType {
    NumberRange(f64, f64),
    VariantList(Vec<String>),
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FieldType::NumberRange(start, end) => write!(f, "{}..{}", start, end),
            FieldType::VariantList(variants) => write!(f, "{}", variants.join(" | ")),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FieldTypes {
    pub pairs: HashMap<String, FieldType>,
}

impl FieldTypes {
    pub fn new() -> Self {
        FieldTypes {
            pairs: HashMap::new(),
        }
    }

    pub fn insert(&mut self, name: String, field_type: FieldType) {
        self.pairs.insert(name, field_type);
    }

    pub fn get(&self, name: &str) -> Option<&FieldType> {
        self.pairs.get(name)
    }

    pub fn iter_names(&self) -> impl Iterator<Item = &String> {
        self.pairs.keys()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&String, &FieldType)> {
        self.pairs.iter()
    }
}

impl Default for FieldTypes {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for FieldTypes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let fields_str: Vec<_> = self
            .pairs
            .iter()
            .map(|(name, field)| format!("{}: {}", name, field))
            .collect();
        write!(f, "{{{}}}", fields_str.join(", "))
    }
}

impl From<Vec<(String, FieldType)>> for FieldTypes {
    fn from(pairs: Vec<(String, FieldType)>) -> Self {
        FieldTypes {
            pairs: pairs.into_iter().collect(),
        }
    }
}

pub enum RelationTypeMapEntry {
    Type(RelationType),
    Complement(String),
}

pub struct RelationTypeMap {
    types: HashMap<String, RelationTypeMapEntry>,
    // I originally included a complement set here to track backwards edges better,
    // but I realized that those will be added as individual types anyways, so
    // they go through all the same validation as forward types eventually. Everything
    // is contained within this code, so there's really no chance of misaligned types.
}

impl RelationTypeMap {
    pub fn new() -> Self {
        RelationTypeMap {
            types: HashMap::new(),
        }
    }

    pub fn from_types(types: Vec<RelationType>) -> Result<Self> {
        let mut mapping = RelationTypeMap::new();
        mapping
            .add_types(types)
            .context("building type mapping from types list")?;
        Ok(mapping)
    }

    /// Gets the type with the given name, if it exists and is a primary type.
    /// Returns None if the name is not found or if it is a complement.
    pub fn get_type(&self, name: &str) -> Option<&RelationType> {
        match self.types.get(name) {
            Some(RelationTypeMapEntry::Type(t)) => Some(t),
            Some(RelationTypeMapEntry::Complement(_)) => None,
            _ => None,
        }
    }

    /// Gets the type with the given name, if it exists. If the name is a
    /// complement, it follows the chain of complements until it finds a
    /// primary type or returns None if it doesn't find one.
    pub fn get_type_or_primary(&self, name: &str) -> Option<&RelationType> {
        match self.types.get(name) {
            Some(RelationTypeMapEntry::Type(t)) => Some(t),
            Some(RelationTypeMapEntry::Complement(other)) => self.get_type_or_primary(other),
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
            Some(RelationTypeMapEntry::Type(t)) => Some(&t.name),
            Some(RelationTypeMapEntry::Complement(other)) => self.get_primary_name(other),
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
            Some(RelationTypeMapEntry::Complement(other)) => Some(other),
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

    pub fn add_types(&mut self, types: Vec<RelationType>) -> Result<()> {
        for t in types {
            let name = t.name.clone();
            self.add_type(t)
                .with_context(|| format!("adding type {}", name))?;
        }
        Ok(())
    }

    pub fn add_type(&mut self, t: RelationType) -> Result<()> {
        self.validate_new_name(&t.name)
            .with_context(|| format!("validating new type name {}", t.name))?;
        match &t.data {
            RelationTypeData::Directional { complement, .. } => {
                self.validate_new_name(complement)
                    .with_context(|| format!("validating complement name {}", complement))?;
                self.types.insert(
                    complement.clone(),
                    RelationTypeMapEntry::Complement(t.name.clone()),
                );
            }
            RelationTypeData::Evaluation { complement } => {
                self.validate_new_name(complement)
                    .with_context(|| format!("validating complement name {}", complement))?;
                self.types.insert(
                    complement.clone(),
                    RelationTypeMapEntry::Complement(t.name.clone()),
                );
            }
            _ => {}
        }
        self.types
            .insert(t.name.clone(), RelationTypeMapEntry::Type(t));
        Ok(())
    }
}
