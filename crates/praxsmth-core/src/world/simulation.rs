use std::collections::HashMap;

use anyhow::{Context, Result, bail};

use crate::{
    anyhow_ext::ResultOptionExt,
    definitions::{
        PraxsmthConstant, PraxsmthValue, Sentence,
        types::{AggregateOp, Effect, Expression, PraxsmthTypeData},
        world::{Declaration, Goal, GoalMeasurement},
    },
    world::{
        AgentToRelation, Bindings, Relation, RelationData, RelationHandle, World,
        transactions::WorldTxn,
    },
};

#[derive(Debug, Clone)]
pub struct ActionRef {
    pub display_name: String,
    pub overall_index: usize,
    pub practice_handle: RelationHandle,
    pub index_within_practice: usize,
}

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
                    .type_mapping
                    .get_type(trait_name)
                    .with_context(|| format!("looking up trait type {}", trait_name))?;
                let PraxsmthTypeData::Trait { .. } = &relation_type.data else {
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
                    .type_mapping
                    .get_type(emotion_name)
                    .with_context(|| format!("looking up emotion type {}", emotion_name))?;
                let PraxsmthTypeData::Emotion { .. } = &relation_type.data else {
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
                    .type_mapping
                    .get_type(practice_name)
                    .with_context(|| format!("looking up practice type {}", practice_name))?;
                let PraxsmthTypeData::Practice { params, .. } = &relation_type.data else {
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
                let relation_type =
                    world
                        .type_mapping
                        .get_type(relation_name)
                        .with_context(|| {
                            format!("looking up binary relation type {}", relation_name)
                        })?;
                match &relation_type.data {
                    PraxsmthTypeData::Directional { .. } => {}
                    PraxsmthTypeData::Reciprocal { .. } => {}
                    PraxsmthTypeData::Evaluation { .. } => {}
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
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone)]
pub struct Dialog {
    pub speaker: Option<String>,
    pub line: String,
}

/// The simulation component of the world, responsible for processing
/// declarations, evaluating variables, and generally doing the work of turning
/// the static world state into a dynamic, interactive simulation. Any write
/// operations must be done through a `WorldTxn` passed into the relevant
/// functions, but read operations can be done directly on the `World`.
///
/// The world state and simulation are tied together through a `PraxsmthApi`.
///
/// There is currently no maintained state... I'm going to keep it as instanced
/// though just in case.
#[derive(Debug, Clone)]
pub struct Simulation {}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulation {
    pub fn new() -> Self {
        Self {}
    }

    /// Adds the information contained within a declaration to the world state.
    ///
    /// The sentence within the declaration must match a query. An error will
    /// be raised if there are any free variables within this query.
    pub fn process_declaration(
        &mut self,
        world: &mut WorldTxn,
        declaration: &Declaration,
        bindings: &Bindings,
    ) -> Result<RelationHandle> {
        let query = Query::parse(world.inner(), &declaration.sentence, bindings)
            .with_context(|| format!("processing declaration {:?}", declaration.sentence))?;

        // TODO: relations with one parameter should be initializable this way!
        let Query::Unfielded(relation_query) = &query else {
            bail!("extra parameters in declaration {:?}", declaration.sentence);
        };

        let relation_query = relation_query.apply_bindings(bindings);

        match relation_query {
            RelationQuery::Trait { agent, trait_name } => {
                world.add_trait(agent.as_literal()?, &trait_name, declaration.fields.clone())
            }
            RelationQuery::Emotion {
                agent,
                emotion_name,
            } => world.add_emotion(
                agent.as_literal()?,
                &emotion_name,
                declaration.fields.clone(),
            ),
            RelationQuery::Binary {
                agent_1,
                agent_2,
                type_name,
            } => world.add_binary_relation(
                agent_1.as_literal()?,
                agent_2.as_literal()?,
                &type_name,
                declaration.fields.clone(),
            ),
            RelationQuery::Practice {
                participants,
                type_name,
            } => world.add_practice(
                participants
                    .iter()
                    .map(AgentRef::as_literal)
                    .collect::<Result<Vec<&str>>>()?,
                &type_name,
                declaration.fields.clone(),
            ),
        }
    }

    /// Uses a relation query to retrieve the associated relation. Will return
    /// an error if there is a free variable in the query.
    ///
    /// Returns `Ok(None)` if the relation specified in the query does not
    /// exist, and `Ok(Some(...))` if it does.
    pub fn lookup_relation<'a>(
        &self,
        world: &'a World,
        query: &RelationQuery,
    ) -> Result<Option<(&'a AgentToRelation, &'a Relation)>> {
        match query {
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

    /// Resolves a fielded query to the value of the specified field. Will
    /// return an error if there is a free variable in the relation query, or
    /// if the specified relation or field does not exist.
    pub fn evaluate_fielded_query(
        &self,
        world: &World,
        relation_query: &RelationQuery,
        field_name: &str,
    ) -> Result<PraxsmthConstant> {
        let (_, relation) = self
            .lookup_relation(world, relation_query)
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

    /// Evaluates a variable (i.e. a sentence) to a constant value. The
    /// sentence must be parsable into a relation query. Bindings will be
    /// applied to the query before evaluation, so free variables can be used
    /// in the sentence as long as they are bound in the provided bindings.
    /// Returns an error if the sentence cannot be parsed into a relation
    /// if there are any free variables in the query after bindings are
    /// applied, or if a fielded query specifies a relation that does not exist
    /// in the world.
    pub fn evaluate_variable(
        &self,
        world: &World,
        sentence: &Sentence,
        bindings: &Bindings,
    ) -> Result<PraxsmthConstant> {
        let query = Query::parse(world, sentence, bindings)?.apply_bindings(bindings);
        match &query {
            Query::Fielded(relation_query, field_name) => {
                // Look into the actual field
                self.evaluate_fielded_query(world, relation_query, field_name)
            }
            Query::Unfielded(relation_query) => {
                // Existence check
                Ok(PraxsmthConstant::Boolean(
                    self.lookup_relation(world, relation_query)?.is_some(),
                ))
            }
        }
    }

    /// Evaluates an expression to a single constant value.
    ///
    /// Returns an error if there are any free variable assignments within the
    /// expression tree. Solve for these with `World::solve_for_free_vars(...)`
    /// first before passing the bindings into this function if you need to
    /// avoid this problem.
    pub fn evaluate_expression(
        &self,
        world: &World,
        expression: &Expression,
        bindings: &Bindings,
    ) -> Result<PraxsmthConstant> {
        match expression {
            Expression::Value(value) => match value {
                PraxsmthValue::Number(n) => Ok(PraxsmthConstant::Number(*n)),
                PraxsmthValue::Boolean(b) => Ok(PraxsmthConstant::Boolean(*b)),
                PraxsmthValue::Variant(v) => Ok(PraxsmthConstant::Variant(v.clone())),
                PraxsmthValue::String(s) => Ok(PraxsmthConstant::String(s.clone())),
                PraxsmthValue::Variable(sentence) => self
                    .evaluate_variable(world, sentence, bindings)
                    .with_context(|| {
                        format!(
                            "evaluating variable for expression with sentence {:?}",
                            sentence
                        )
                    }),
            },

            Expression::And(x, y) => {
                let x = self.evaluate_expression(world, x.as_ref(), bindings)?;
                let PraxsmthConstant::Boolean(x) = x else {
                    bail!("And conditions must evaluate to boolean, got {:?}", x);
                };
                if !x {
                    // Short circuit, important behavior!
                    return Ok(PraxsmthConstant::Boolean(false));
                }

                let y = self.evaluate_expression(world, y.as_ref(), bindings)?;
                match y {
                    PraxsmthConstant::Boolean(y) => Ok(PraxsmthConstant::Boolean(y)),
                    other => bail!("And conditions must evaluate to boolean, got {:?}", other),
                }
            }

            Expression::Or(x, y) => {
                let x = self.evaluate_expression(world, x.as_ref(), bindings)?;
                let PraxsmthConstant::Boolean(x) = x else {
                    bail!("Or conditions must evaluate to boolean, got {:?}", x);
                };
                if x {
                    // Short circuit, important behavior!
                    return Ok(PraxsmthConstant::Boolean(true));
                }

                let y = self.evaluate_expression(world, y.as_ref(), bindings)?;
                match y {
                    PraxsmthConstant::Boolean(y) => Ok(PraxsmthConstant::Boolean(y)),
                    other => bail!("Or conditions must evaluate to boolean, got {:?}", other),
                }
            }

            Expression::Is(x, y) => {
                let x = self.evaluate_expression(world, x.as_ref(), bindings)?;
                let y = self.evaluate_expression(world, y.as_ref(), bindings)?;
                Ok(PraxsmthConstant::Boolean(x == y))
            }

            Expression::Not(x) => {
                let res = self.evaluate_expression(world, x.as_ref(), bindings)?;
                match res {
                    PraxsmthConstant::Boolean(b) => Ok(PraxsmthConstant::Boolean(!b)),
                    other => bail!("Not condition must evaluate to boolean, got {:?}", other),
                }
            }

            Expression::ForAll(new_symbol, inner) => {
                for (agent_id, _) in world.iter_agents() {
                    let new_bindings =
                        bindings.with([(new_symbol.clone(), agent_id.clone())].into());
                    match self.evaluate_expression(world, inner.as_ref(), &new_bindings)? {
                        PraxsmthConstant::Boolean(true) => continue,
                        PraxsmthConstant::Boolean(false) => {
                            return Ok(PraxsmthConstant::Boolean(false));
                        }
                        other => {
                            bail!("ForAll condition must evaluate to boolean, got {:?}", other)
                        }
                    }
                }
                Ok(PraxsmthConstant::Boolean(true))
            }

            Expression::Any(new_symbol, inner) => {
                for (agent_id, _) in world.iter_agents() {
                    let new_bindings =
                        bindings.with([(new_symbol.clone(), agent_id.clone())].into());
                    match self.evaluate_expression(world, inner.as_ref(), &new_bindings)? {
                        PraxsmthConstant::Boolean(true) => {
                            return Ok(PraxsmthConstant::Boolean(true));
                        }
                        PraxsmthConstant::Boolean(false) => continue,
                        other => {
                            bail!("Any condition must evaluate to boolean, got {:?}", other)
                        }
                    }
                }
                Ok(PraxsmthConstant::Boolean(false))
            }

            Expression::Count(new_symbol, inner) => {
                let mut count = 0;
                for (agent_id, _) in world.iter_agents() {
                    let new_bindings =
                        bindings.with([(new_symbol.clone(), agent_id.clone())].into());
                    match self.evaluate_expression(world, inner.as_ref(), &new_bindings)? {
                        PraxsmthConstant::Boolean(true) => count += 1,
                        PraxsmthConstant::Boolean(false) => continue,
                        other => {
                            bail!("Count condition must evaluate to boolean, got {:?}", other)
                        }
                    }
                }
                Ok(PraxsmthConstant::Number(count.into()))
            }

            Expression::Aggregate {
                op,
                body,
                var,
                filter,
            } => {
                let mut values = vec![];

                for (agent_id, _) in world.iter_agents() {
                    let new_bindings = bindings.with([(var.clone(), agent_id.clone())].into());
                    match self.evaluate_expression(world, filter.as_ref(), &new_bindings)? {
                        PraxsmthConstant::Boolean(true) => {
                            let value =
                                self.evaluate_expression(world, body.as_ref(), &new_bindings)?;
                            match value {
                                PraxsmthConstant::Number(n) => values.push(n),
                                other => {
                                    bail!("Aggregate body must evaluate to number, got {:?}", other)
                                }
                            }
                        }
                        PraxsmthConstant::Boolean(false) => continue,
                        other => {
                            bail!("Aggregate filter must evaluate to boolean, got {:?}", other)
                        }
                    }
                }

                Ok(match op {
                    AggregateOp::Sum => PraxsmthConstant::Number(values.into_iter().sum()),
                    AggregateOp::Average => {
                        let count = values.len();
                        if count == 0 {
                            PraxsmthConstant::Number(0.0)
                        } else {
                            PraxsmthConstant::Number(values.into_iter().sum::<f64>() / count as f64)
                        }
                    }
                    AggregateOp::Min => values
                        .into_iter()
                        .min_by(|a, b| a.partial_cmp(b).unwrap())
                        .map(PraxsmthConstant::Number)
                        .unwrap_or(PraxsmthConstant::Number(0.0)),
                    AggregateOp::Max => values
                        .into_iter()
                        .max_by(|a, b| a.partial_cmp(b).unwrap())
                        .map(PraxsmthConstant::Number)
                        .unwrap_or(PraxsmthConstant::Number(0.0)),
                })
            }
        }
    }

    pub fn check_condition(
        &self,
        world: &World,
        expression: &Expression,
        bindings: &Bindings,
    ) -> Result<bool> {
        match self.evaluate_expression(world, expression, bindings)? {
            PraxsmthConstant::Boolean(b) => Ok(b),
            other => bail!(
                "condition expression must evaluate to boolean, got {:?}",
                other
            ),
        }
    }

    fn process_print(
        &mut self,
        world: &World,
        speaker: Option<&str>,
        string: &str,
        bindings: &Bindings,
    ) -> Result<Dialog> {
        let dialog = Dialog {
            speaker: speaker.map(|s| s.to_string()),
            line: world.format_string(string, bindings).with_context(|| {
                format!(
                    "formatting string for print outcome with speaker {:?}: {}",
                    speaker, string
                )
            })?,
        };
        Ok(dialog)
    }

    fn process_delete(
        &mut self,
        world: &mut WorldTxn,
        sentence: &Sentence,
        bindings: &Bindings,
    ) -> Result<()> {
        let query = Query::parse(world.inner(), sentence, bindings)
            .with_context(|| format!("processing delete outcome {:?}", sentence))?;

        let Query::Unfielded(relation_query) = &query else {
            bail!("extra parameters in delete outcome {:?}", sentence);
        };

        let relation_query = relation_query.apply_bindings(bindings);

        let (edge, _) = self
            .lookup_relation(world.inner(), &relation_query)
            .require_with_context(|| {
                format!("relation not found in delete outcome {:?}", sentence)
            })?;
        world
            .remove_relation(edge.relation_handle.clone())
            .with_context(|| format!("removing relation in delete outcome {:?}", sentence))
    }

    fn process_update(
        &mut self,
        world: &mut WorldTxn,
        sentence: &Sentence,
        value: &PraxsmthValue,
        bindings: &Bindings,
    ) -> Result<()> {
        let query = Query::parse(world.inner(), sentence, bindings)
            .with_context(|| format!("processing update outcome {:?}", sentence))?;
        let Query::Fielded(relation_query, field_name) = &query else {
            // TODO: Support bracket syntax so this is actually usable!!!
            // (Currently you can't initialize anything with >1 fields)
            // Just needs to be added to parser and the logic flow here,
            // the update functions should already support it.
            bail!(
                "update outcome must specify a field to update {:?}",
                sentence
            );
        };

        let relation_query = relation_query.apply_bindings(bindings);

        let (edge, _) = self
            .lookup_relation(world.inner(), &relation_query)
            .require_with_context(|| {
                format!("relation not found in update outcome {:?}", sentence)
            })?;

        let constant_value = match value {
            PraxsmthValue::Number(n) => PraxsmthConstant::Number(*n),
            PraxsmthValue::Boolean(b) => PraxsmthConstant::Boolean(*b),
            PraxsmthValue::Variant(v) => PraxsmthConstant::Variant(v.clone()),
            PraxsmthValue::String(s) => PraxsmthConstant::String(s.clone()),
            PraxsmthValue::Variable(new_val_sentence) => self
                .evaluate_variable(world.inner(), new_val_sentence, bindings)
                .with_context(|| {
                    format!(
                        "evaluating variable for new value in update outcome with sentence {:?}",
                        new_val_sentence
                    )
                })?,
        };

        world
            .update_relation(
                edge.relation_handle.clone(),
                HashMap::from([(field_name.clone(), constant_value)]),
            )
            .with_context(|| format!("applying update outcome {:?}", sentence))
    }

    fn process_increase(
        &mut self,
        _world: &mut WorldTxn,
        _sentence: &Sentence,
        _amount: i64,
        _bindings: &Bindings,
    ) -> Result<()> {
        unimplemented!()
    }

    fn process_cycle(
        &mut self,
        _world: &mut WorldTxn,
        _sentence: &Sentence,
        _amount: i64,
        _bindings: &Bindings,
    ) -> Result<()> {
        unimplemented!()
    }

    pub fn process_effect(
        &mut self,
        world: &mut WorldTxn,
        agent_name: &str,
        effect: &Effect,
        bindings: &Bindings,
    ) -> Result<Option<Dialog>> {
        match effect {
            Effect::Broadcast(string) => {
                return Ok(Some(self.process_print(
                    world.inner(),
                    None,
                    string,
                    bindings,
                )?));
            }
            Effect::Say(string) => {
                return Ok(Some(self.process_print(
                    world.inner(),
                    Some(agent_name),
                    string,
                    bindings,
                )?));
            }
            Effect::Activate(agent_id) => {
                world.set_agent_active(&bindings.get_or_same(agent_id), true)
            }
            Effect::Deactivate(agent_id) => {
                world.set_agent_active(&bindings.get_or_same(agent_id), false)
            }
            Effect::Delete(sentence) => self.process_delete(world, sentence, bindings),
            Effect::Set(declaration) => self
                .process_declaration(world, declaration, bindings)
                .map(|_| ()),
            Effect::Update(sentence, value) => {
                self.process_update(world, sentence, value, bindings)
            }
            Effect::Increase(sentence, amount) => {
                self.process_increase(world, sentence, *amount, bindings)
            }
            Effect::Cycle(sentence, amount) => {
                self.process_cycle(world, sentence, *amount, bindings)
            }
        }?;
        Ok(None)
    }

    pub fn get_available_actions(&self, world: &World, agent_name: &str) -> Result<Vec<ActionRef>> {
        let agent = world
            .get_agent(agent_name)
            .with_context(|| format!("agent {} not found", agent_name))?;
        let mut available_actions = Vec::new();

        for (edge, relation) in world.iter_agent_relations(agent) {
            match &relation.data {
                RelationData::Practice { bindings, .. } => {
                    let relation_type = world
                        .type_mapping
                        .get_type(&relation.type_name)
                        .with_context(|| {
                            format!("type {} not found for practice action", relation.type_name)
                        })?;
                    let PraxsmthTypeData::Practice { actions, .. } = &relation_type.data else {
                        bail!(
                            "type {} data is not practice for action lookup",
                            relation.type_name
                        );
                    };
                    'action_loop: for (i, action) in actions.iter().enumerate() {
                        let action_for =
                            World::resolve_binding_or_same(&action.for_actor, bindings);
                        if action_for != agent_name {
                            continue;
                        }
                        for condition in &action.conditions {
                            if !self
                                .check_condition(world, condition, bindings)
                                .with_context(|| {
                                    format!(
                                        "checking conditions for action {} of practice {:?}",
                                        action.name, relation_type.name
                                    )
                                })?
                            {
                                continue 'action_loop;
                            }
                        }

                        available_actions.push(ActionRef {
                            display_name: world
                                .format_string(&action.name, bindings)
                                .with_context(|| {
                                    format!(
                                        "formatting display name for action {} of practice {:?}",
                                        action.name, relation_type.name
                                    )
                                })?,
                            overall_index: available_actions.len(),
                            practice_handle: edge.relation_handle.clone(),
                            index_within_practice: i,
                        });
                    }
                }
                _ => {}
            }
        }

        Ok(available_actions)
    }

    pub fn process_available_action(
        &mut self,
        world: &mut WorldTxn,
        available_action: &ActionRef,
    ) -> Result<Vec<Dialog>> {
        let relation = world
            .inner()
            .get_relation(available_action.practice_handle.clone())
            .with_context(|| {
                format!(
                    "relation {:?} not found for available action",
                    available_action.practice_handle
                )
            })?;
        let RelationData::Practice { bindings, .. } = &relation.data else {
            bail!(
                "relation {:?} data is not practice for available action",
                available_action.practice_handle
            );
        };
        let relation_type = world
            .inner()
            .type_mapping
            .get_type(&relation.type_name)
            .with_context(|| {
                format!("type {} not found for available action", relation.type_name)
            })?;
        let PraxsmthTypeData::Practice { actions, .. } = &relation_type.data else {
            bail!(
                "type {} is not a practice for available action",
                relation.type_name
            );
        };
        let action = actions
            .get(available_action.index_within_practice)
            .with_context(|| {
                format!(
                    "action index {} out of bounds for practice {:?}",
                    available_action.index_within_practice, available_action.practice_handle
                )
            })?;

        // TODO: Fix this a better way.
        let actor_name = World::resolve_binding_or_same(&action.for_actor, &bindings);
        let effects = action.effects.clone();
        let action_name = action.name.clone();
        let bindings = bindings.clone();

        let mut dialog: Vec<Dialog> = vec![];

        for effect in &effects {
            if let Some(new_dialog) = self
                .process_effect(world, &actor_name, effect, &bindings)
                .with_context(|| format!("processing effect of action {}", action_name))?
            {
                dialog.push(new_dialog);
            }
        }

        Ok(dialog)
    }

    /// Evaluates the current state of a goal within the context of a world.
    /// This value is not entirely useful by itself, and is intended to be used
    /// as part of a net delta of an agent's goals across two world states. The
    /// return value is derived from the goal's measurements and weights.
    ///
    /// This system supports the same unbound variables as conditions do, and
    /// any expressions with multiple possible bindings will have their weights
    /// summed.
    ///
    /// WARNING: If a new edge is added that gets caught by an increase
    /// measurement, it will result in a huge delta for that event. I would
    /// recommend normalizing your values for this; throwing away non-delta
    /// scores would fix this issue, but also throws away potentially useful
    /// information, so I've decided not to do that.
    fn evaluate_goal(&self, world: &World, goal: &Goal, bindings: &Bindings) -> Result<f64> {
        let evaluation = self.evaluate_expression(world, &goal.expression, bindings)?;

        let mut total_weight = 0.0;
        match goal.measurement {
            GoalMeasurement::Exists => match evaluation {
                PraxsmthConstant::Boolean(b) => {
                    if b {
                        total_weight += goal.weight;
                    }
                }
                other => bail!(
                    "goal expression must evaluate to boolean for Exists measurement, got {:?}",
                    other
                ),
            },
            GoalMeasurement::Delta => match evaluation {
                PraxsmthConstant::Number(n) => {
                    total_weight += n * goal.weight;
                }
                other => bail!(
                    "goal expression must evaluate to number for Increase/Decrease measurement, got {:?}",
                    other
                ),
            },
        }

        Ok(total_weight)
    }

    /// Evaluates all of an agent's goals and returns the total score. This is
    /// intended to be used as part of a net delta of an agent's goals across
    /// two world states, so the return value is not entirely useful by itself.
    /// The return value is derived from the goals' measurements and weights.
    pub fn evaluate_agent_goals(&self, world: &World, agent_id: &str) -> Result<f64> {
        let agent = world
            .get_agent(agent_id)
            .with_context(|| format!("agent {} not found for goal evaluation", agent_id))?;
        let mut total_score = 0.0;
        for goal in &agent.goals {
            total_score += self.evaluate_goal(world, goal, &Bindings::default())?;
        }
        Ok(total_score)
    }
}
