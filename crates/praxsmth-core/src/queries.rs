use anyhow::{Context, Result, bail};

use crate::{
    anyhow_ext::ResultOptionExt,
    types::RelationTypeData,
    values::{Constant, Sentence},
    world::{AgentToRelation, Relation, World, bindings::Bindings},
};

#[derive(Debug, Clone)]
pub enum Query {
    Fielded(RelationQuery, String),
    Unfielded(RelationQuery),
}

impl Query {
    pub fn relation_query(&self) -> &RelationQuery {
        match self {
            Query::Fielded(relation_query, _) => relation_query,
            Query::Unfielded(relation_query) => relation_query,
        }
    }

    pub fn try_new_with_fields(relation_query: RelationQuery, fields: &[String]) -> Result<Self> {
        if fields.len() > 1 {
            bail!(
                "too many fields specified for relation query {:?}, got {}",
                relation_query,
                fields.len()
            );
        } else if fields.len() == 1 {
            Ok(Query::Fielded(relation_query, fields[0].clone()))
        } else {
            Ok(Query::Unfielded(relation_query))
        }
    }

    pub fn parse(world: &World, sentence: &Sentence, bindings: &Bindings) -> Result<Self> {
        match sentence.as_slice() {
            [self_keyword, rest @ ..] if self_keyword == "self" => {
                let self_sentence = bindings
                    .self_id
                    .as_ref()
                    .with_context(|| "sentence starting with 'self' has no self context")?;
                // Can't simply recurse because we would lose rest, so just recurse to
                // build the query for the self context and then re-attach the rest.
                let query = Self::parse(world, self_sentence, bindings).with_context(|| {
                    format!(
                        "parsing sentence starting with 'self' using self context {:?}",
                        self_sentence
                    )
                })?;
                Query::try_new_with_fields(query.relation_query().clone(), rest)
            }
            [agent, verb, trait_name, rest @ ..] if verb == "is" => {
                let relation_type = world
                    .get_type_mapping()
                    .get_type(trait_name)
                    .with_context(|| format!("looking up trait type {}", trait_name))?;
                let RelationTypeData::Trait { .. } = &relation_type.data else {
                    bail!("type {} is not a trait", trait_name);
                };
                Query::try_new_with_fields(
                    RelationQuery::Trait {
                        agent: AgentRef::new(agent, bindings)?,
                        trait_name: trait_name.clone(),
                    },
                    rest,
                )
            }
            [agent, verb, emotion_name, rest @ ..] if verb == "feels" => {
                let relation_type = world
                    .get_type_mapping()
                    .get_type(emotion_name)
                    .with_context(|| format!("looking up emotion type {}", emotion_name))?;
                let RelationTypeData::Emotion { .. } = &relation_type.data else {
                    bail!("type {} is not an emotion", emotion_name);
                };
                Query::try_new_with_fields(
                    RelationQuery::Emotion {
                        agent: AgentRef::new(agent, bindings)?,
                        emotion_name: emotion_name.clone(),
                    },
                    rest,
                )
            }
            [practice, practice_name, rest @ ..] if practice == "practice" => {
                let relation_type = world
                    .get_type_mapping()
                    .get_type(practice_name)
                    .with_context(|| format!("looking up practice type {}", practice_name))?;
                let RelationTypeData::Practice { params, .. } = &relation_type.data else {
                    bail!("type {} is not a practice", practice_name);
                };
                let participants_count = params.len();
                if rest.len() < participants_count {
                    bail!(
                        "practice {} expects {} participants, got {}",
                        practice_name,
                        participants_count,
                        rest.len()
                    );
                }
                let participants = rest[..participants_count]
                    .iter()
                    .map(|a| AgentRef::new(a, bindings))
                    .collect::<Result<Vec<AgentRef>>>()?;
                Query::try_new_with_fields(
                    RelationQuery::Practice {
                        participants,
                        type_name: practice_name.clone(),
                    },
                    &rest[participants_count..],
                )
            }
            [agent_1, relation_name, agent_2, rest @ ..] => {
                let relation_type = world
                    .get_type_mapping()
                    .get_type(relation_name)
                    .with_context(|| {
                        format!("looking up binary relation type {}", relation_name)
                    })?;
                match &relation_type.data {
                    RelationTypeData::Directional { .. } => {}
                    RelationTypeData::Reciprocal { .. } => {}
                    RelationTypeData::Evaluation { .. } => {}
                    _ => bail!("type {} is not a binary relation", relation_name),
                }
                Query::try_new_with_fields(
                    RelationQuery::Binary {
                        agent_1: AgentRef::new(agent_1, bindings)?,
                        agent_2: AgentRef::new(agent_2, bindings)?,
                        type_name: relation_name.clone(),
                    },
                    rest,
                )
            }
            _ => bail!(
                "could not parse sentence {:?} into a relation query",
                sentence
            ),
        }
    }

    pub fn get_agent_refs(&self) -> Vec<&AgentRef> {
        match self.relation_query() {
            RelationQuery::Trait { agent, .. } => vec![agent],
            RelationQuery::Emotion { agent, .. } => vec![agent],
            RelationQuery::Binary {
                agent_1, agent_2, ..
            } => vec![agent_1, agent_2],
            RelationQuery::Practice { participants, .. } => participants.iter().collect(),
        }
    }

    pub fn is_any_agent_free(&self) -> bool {
        self.get_agent_refs()
            .iter()
            .any(|agent_ref| agent_ref.is_free())
    }

    pub fn apply_bindings(&self, bindings: &Bindings) -> Self {
        match self {
            Query::Fielded(relation_query, field_name) => {
                Query::Fielded(relation_query.apply_bindings(bindings), field_name.clone())
            }
            Query::Unfielded(relation_query) => {
                Query::Unfielded(relation_query.apply_bindings(bindings))
            }
        }
    }

    pub fn evaluate_in_world(&self, world: &World) -> Result<Constant> {
        match self {
            Query::Fielded(relation_query, field_name) => {
                // Look into the actual field
                let (_, relation) =
                    relation_query
                        .lookup_in_world(world)
                        .require_with_context(|| {
                            format!(
                                "evaluating query for relation {:?} with field {:?}",
                                relation_query, field_name
                            )
                        })?;

                // An argument was specified, so pull it from the relation's fields
                relation
                    .fields
                    .get(field_name)
                    .cloned()
                    .with_context(|| format!("field '{}' not found in relation", field_name))
            }
            Query::Unfielded(relation_query) => {
                // Existence check
                Ok(Constant::Boolean(
                    relation_query.lookup_in_world(world)?.is_some(),
                ))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RelationQuery {
    Trait {
        agent: AgentRef,
        trait_name: String,
    },
    Emotion {
        agent: AgentRef,
        emotion_name: String,
    },
    Binary {
        agent_1: AgentRef,
        agent_2: AgentRef,
        type_name: String,
    },
    Practice {
        participants: Vec<AgentRef>,
        type_name: String,
    },
}

impl RelationQuery {
    pub fn apply_bindings(&self, bindings: &Bindings) -> Self {
        match self {
            RelationQuery::Trait { agent, trait_name } => RelationQuery::Trait {
                agent: agent.bind_or_same(bindings),
                trait_name: trait_name.clone(),
            },
            RelationQuery::Emotion {
                agent,
                emotion_name,
            } => RelationQuery::Emotion {
                agent: agent.bind_or_same(bindings),
                emotion_name: emotion_name.clone(),
            },
            RelationQuery::Binary {
                agent_1,
                agent_2,
                type_name,
            } => RelationQuery::Binary {
                agent_1: agent_1.bind_or_same(bindings),
                agent_2: agent_2.bind_or_same(bindings),
                type_name: type_name.clone(),
            },
            RelationQuery::Practice {
                participants,
                type_name,
            } => RelationQuery::Practice {
                participants: participants
                    .iter()
                    .map(|p| p.bind_or_same(bindings))
                    .collect(),
                type_name: type_name.clone(),
            },
        }
    }

    /// Uses a relation query to retrieve the associated relation. Will return
    /// an error if there is a free variable in the query.
    ///
    /// Returns `Ok(None)` if the relation specified in the query does not
    /// exist, and `Ok(Some(...))` if it does.
    pub fn lookup_in_world<'a>(
        &self,
        world: &'a World,
    ) -> Result<Option<(&'a AgentToRelation, &'a Relation)>> {
        match self {
            RelationQuery::Trait { agent, trait_name } => {
                let agent_lit = agent.as_literal()?;
                world.get_trait(agent_lit, &trait_name).with_context(|| {
                    format!(
                        "could not find trait with agent: {}, trait name: {}",
                        agent_lit, trait_name
                    )
                })
            }
            RelationQuery::Emotion {
                agent,
                emotion_name,
            } => {
                let agent_lit = agent.as_literal()?;
                world
                    .get_emotion(agent_lit, &emotion_name)
                    .with_context(|| {
                        format!(
                            "could not find emotion with agent: {}, emotion name: {}",
                            agent_lit, emotion_name
                        )
                    })
            }
            RelationQuery::Binary {
                agent_1,
                agent_2,
                type_name,
            } => {
                let agent_1_lit = agent_1.as_literal()?;
                let agent_2_lit = agent_2.as_literal()?;
                world.get_binary_relation(agent_1_lit, agent_2_lit, &type_name).with_context(|| {
                    format!(
                        "could not find binary relation with agent 1: {}, agent 2: {}, type name: {}",
                        agent_1_lit, agent_2_lit, type_name
                    )
                })
            }
            RelationQuery::Practice {
                participants,
                type_name,
            } => {
                let participants_lit = participants
                    .iter()
                    .map(AgentRef::as_literal)
                    .collect::<Result<Vec<&str>>>()?;
                world
                    .get_practice(participants_lit, &type_name)
                    .with_context(|| {
                        format!(
                            "could not find practice with participants: {:?}, practice name: {}",
                            participants, type_name
                        )
                    })
            }
        }
    }

    pub fn get_all_agents(&self) -> Vec<&AgentRef> {
        match self {
            RelationQuery::Trait { agent, .. } => vec![agent],
            RelationQuery::Emotion { agent, .. } => vec![agent],
            RelationQuery::Binary {
                agent_1, agent_2, ..
            } => vec![agent_1, agent_2],
            RelationQuery::Practice { participants, .. } => participants.iter().collect(),
        }
    }

    pub fn type_name(&self) -> &str {
        match self {
            RelationQuery::Trait { trait_name, .. } => trait_name,
            RelationQuery::Emotion { emotion_name, .. } => emotion_name,
            RelationQuery::Binary { type_name, .. } => type_name,
            RelationQuery::Practice { type_name, .. } => type_name,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentRef {
    Literal(String),
    Free(String),
}

impl AgentRef {
    pub fn new(specifier: &str, bindings: &Bindings) -> Result<AgentRef> {
        let first_char = &specifier
            .chars()
            .nth(0)
            .with_context(|| "agent ref could not be built from an empty specifier")?;
        match bindings.get(specifier) {
            Some(id) => Ok(AgentRef::Literal(id.into())),
            None => {
                if first_char.is_ascii_uppercase() {
                    Ok(AgentRef::Free(specifier.into()))
                } else {
                    Ok(AgentRef::Literal(specifier.into()))
                }
            }
        }
    }

    pub fn as_literal(&self) -> Result<&str> {
        match self {
            Self::Literal(id) => Ok(id),
            Self::Free(specifier) => bail!(format!(
                "agent ref {} is an unbound free variable",
                specifier
            )),
        }
    }

    pub fn is_free(&self) -> bool {
        matches!(self, Self::Free(_))
    }

    pub fn bind_or_same(&self, bindings: &Bindings) -> AgentRef {
        match self {
            Self::Literal(_) => self.clone(),
            Self::Free(specifier) => match bindings.get(specifier) {
                Some(id) => AgentRef::Literal(id.into()),
                None => self.clone(),
            },
        }
    }

    pub fn symbol(&self) -> &str {
        match self {
            Self::Literal(id) => id,
            Self::Free(specifier) => specifier,
        }
    }
}
