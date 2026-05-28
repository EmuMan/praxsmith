use std::{collections::HashMap, fmt};

use anyhow::{Context, Result, bail};

use crate::{expressions::Expression, values::Sentence, world::simulation::Effect};

pub mod checking;

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
            RelationTypeData::Directional { exclusive } => write!(
                f,
                "{}directional {} {{{}}}",
                if *exclusive { "exclusive " } else { "" },
                self.name,
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
        exclusive: bool,
    },
    Reciprocal,
    Emotion,
    Practice {
        self_id: Sentence,
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
    ActorRef,
    String,
    Boolean,
}

impl fmt::Display for FieldType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FieldType::NumberRange(start, end) => write!(f, "{}..{}", start, end),
            FieldType::VariantList(variants) => write!(f, "{}", variants.join(" | ")),
            FieldType::ActorRef => write!(f, "ActorRef"),
            FieldType::String => write!(f, "String"),
            FieldType::Boolean => write!(f, "Boolean"),
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

pub struct RelationTypeMap {
    types: HashMap<String, RelationType>,
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

    /// Gets the type with the given name, if it exists.
    /// Returns None if the name is not found.
    pub fn get_type(&self, name: &str) -> Option<&RelationType> {
        self.types.get(name)
    }

    pub fn validate_new_name(&self, name: &str) -> Result<()> {
        if let Some(existing) = self.get_type(name) {
            bail!("some type with name {} already exists: {}", name, existing);
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
        self.types.insert(t.name.clone(), t);
        Ok(())
    }

    pub fn iter_types(&self) -> impl Iterator<Item = &RelationType> {
        self.types.values()
    }
}
