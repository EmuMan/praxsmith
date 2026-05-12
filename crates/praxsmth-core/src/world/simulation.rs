use std::collections::HashMap;

use anyhow::{Context, Result, bail};

use crate::{
    definitions::{
        PraxsmthConstant, PraxsmthValue, Sentence,
        types::{Condition, PracticeOutcome, PraxsmthTypeData},
        world::Declaration,
    },
    world::{AgentToRelation, Bindings, Relation, RelationData, RelationHandle, World},
};

#[derive(Debug, Clone)]
pub struct AvailableAction {
    pub display_name: String,
    pub overall_index: usize,
    pub practice_handle: RelationHandle,
    pub index_within_practice: usize,
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
}

pub struct Dialog {
    pub speaker: Option<String>,
    pub line: String,
}

impl World {
    /// Parses a sentence into a relation query, returning the query
    /// and any remaining arguments. Verifies the type of the relation
    /// and the number of parameters, but does not verify the agents
    /// involved.
    pub fn build_query(
        &self,
        sentence: &Sentence,
        bindings: &Bindings,
    ) -> Result<(RelationQuery, Box<[String]>)> {
        match sentence.as_slice() {
            [self_keyword, rest @ ..] if self_keyword == "self" => {
                let self_sentence = bindings
                    .self_id
                    .as_ref()
                    .with_context(|| "sentence starting with 'self' has no self context")?;
                // Can't simply recurse because we would lose rest, so just recurse to
                // build the query for the self context and then re-attach the rest.
                let (query, _) = self.build_query(self_sentence, bindings).with_context(|| {
                    format!(
                        "parsing sentence starting with 'self' using self context {:?}",
                        self_sentence
                    )
                })?;
                Ok((query, rest.into()))
            }
            [agent, verb, trait_name, rest @ ..] if verb == "is" => {
                let relation_type = self
                    .type_mapping
                    .get_type(trait_name)
                    .with_context(|| format!("looking up trait type {}", trait_name))?;
                let PraxsmthTypeData::Trait { .. } = &relation_type.data else {
                    bail!("type {} is not a trait", trait_name);
                };
                Ok((
                    RelationQuery::Trait {
                        agent: AgentRef::new(agent, bindings)?,
                        trait_name: trait_name.clone(),
                    },
                    rest.into(),
                ))
            }
            [agent, verb, emotion_name, rest @ ..] if verb == "feels" => {
                let relation_type = self
                    .type_mapping
                    .get_type(emotion_name)
                    .with_context(|| format!("looking up emotion type {}", emotion_name))?;
                let PraxsmthTypeData::Emotion { .. } = &relation_type.data else {
                    bail!("type {} is not an emotion", emotion_name);
                };
                Ok((
                    RelationQuery::Emotion {
                        agent: AgentRef::new(agent, bindings)?,
                        emotion_name: emotion_name.clone(),
                    },
                    rest.into(),
                ))
            }
            [practice, practice_name, rest @ ..] if practice == "practice" => {
                let relation_type = self
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
                Ok((
                    RelationQuery::Practice {
                        participants: participants,
                        type_name: practice_name.clone(),
                    },
                    rest[participants_count..].into(),
                ))
            }
            [agent_1, relation_name, agent_2, rest @ ..] => {
                let relation_type =
                    self.type_mapping.get_type(relation_name).with_context(|| {
                        format!("looking up binary relation type {}", relation_name)
                    })?;
                match &relation_type.data {
                    PraxsmthTypeData::Directional { .. } => {}
                    PraxsmthTypeData::Reciprocal { .. } => {}
                    PraxsmthTypeData::Evaluation { .. } => {}
                    _ => bail!("type {} is not a binary relation", relation_name),
                }
                Ok((
                    RelationQuery::Binary {
                        agent_1: AgentRef::new(agent_1, bindings)?,
                        agent_2: AgentRef::new(agent_2, bindings)?,
                        type_name: relation_name.clone(),
                    },
                    rest.into(),
                ))
            }
            _ => bail!(
                "could not parse sentence {:?} into a relation query",
                sentence
            ),
        }
    }

    pub fn process_declaration(
        &mut self,
        declaration: &Declaration,
        bindings: &Bindings,
    ) -> Result<RelationHandle> {
        let (query, args) = self
            .build_query(&declaration.sentence, bindings)
            .with_context(|| format!("processing declaration {:?}", declaration.sentence))?;
        // TODO: relations with one parameter should be initializable this way!
        if !args.is_empty() {
            bail!("extra parameters in declaration {:?}", declaration.sentence);
        }
        match query {
            RelationQuery::Trait { agent, trait_name } => {
                self.add_trait(agent.as_literal()?, &trait_name, declaration.fields.clone())
            }
            RelationQuery::Emotion {
                agent,
                emotion_name,
            } => self.add_emotion(
                agent.as_literal()?,
                &emotion_name,
                declaration.fields.clone(),
            ),
            RelationQuery::Binary {
                agent_1,
                agent_2,
                type_name,
            } => self.add_binary_relation(
                agent_1.as_literal()?,
                agent_2.as_literal()?,
                &type_name,
                declaration.fields.clone(),
            ),
            RelationQuery::Practice {
                participants,
                type_name,
            } => self.add_practice(
                participants
                    .iter()
                    .map(AgentRef::as_literal)
                    .collect::<Result<Vec<&str>>>()?,
                &type_name,
                declaration.fields.clone(),
            ),
        }
    }

    pub fn lookup_relation(&self, query: RelationQuery) -> Result<(RelationHandle, &Relation)> {
        match query {
            RelationQuery::Trait { agent, trait_name } => {
                let agent_lit = agent.as_literal()?;
                self.get_trait(agent_lit, &trait_name).with_context(|| {
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
                self.get_emotion(agent_lit, &emotion_name).with_context(|| {
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
                self.get_binary_relation(agent_1_lit, agent_2_lit, &type_name).with_context(|| {
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
                self.get_practice(participants_lit, &type_name)
                    .with_context(|| {
                        format!(
                            "could not find practice with participants: {:?}, practice name: {}",
                            participants, type_name
                        )
                    })
            }
        }
    }

    pub fn resolve_variable(
        &self,
        sentence: &Sentence,
        bindings: &Bindings,
    ) -> Result<PraxsmthConstant> {
        let (query, args) = self
            .build_query(&sentence, bindings)
            .with_context(|| format!("resolving variable {:?}", sentence))?;

        // TODO: propagate relation not found, but NOT free variable errors!!!!
        let Ok((_, relation)) = self.lookup_relation(query) else {
            // No relation found, so this evaluates to false
            return Ok(PraxsmthConstant::Boolean(false));
        };

        if args.is_empty() {
            // No parameters specified but there is a relationship, so return true
            return Ok(PraxsmthConstant::Boolean(true));
        }

        if args.len() > 1 {
            bail!("too many parameters specified in variable {:?}", sentence);
        }

        // An argument was specified, so pull it from the relation's fields
        let arg_name = &args[0];
        relation
            .fields
            .get(arg_name)
            .cloned()
            .with_context(|| format!("parameter '{}' not found in relation", arg_name))
    }

    fn check_condition_helper(
        &self,
        condition: Condition,
        bindings: &Bindings,
    ) -> Result<PraxsmthConstant> {
        match condition {
            Condition::Value(value) => match value {
                PraxsmthValue::Number(n) => Ok(PraxsmthConstant::Number(n)),
                PraxsmthValue::Boolean(b) => Ok(PraxsmthConstant::Boolean(b)),
                PraxsmthValue::Variant(v) => Ok(PraxsmthConstant::Variant(v)),
                PraxsmthValue::String(s) => Ok(PraxsmthConstant::String(s)),
                PraxsmthValue::Variable(sentence) => self.resolve_variable(&sentence, bindings),
            },

            Condition::And(cond_1, cond_2) => {
                let res_1 = self.check_condition_helper(*cond_1, bindings)?;
                let res_2 = self.check_condition_helper(*cond_2, bindings)?;
                match (res_1, res_2) {
                    (PraxsmthConstant::Boolean(b1), PraxsmthConstant::Boolean(b2)) => {
                        Ok(PraxsmthConstant::Boolean(b1 && b2))
                    }
                    (a, b) => bail!(
                        "And conditions must evaluate to boolean, got {:?} and {:?}",
                        a,
                        b
                    ),
                }
            }

            Condition::Or(cond_1, cond_2) => {
                let res_1 = self.check_condition_helper(*cond_1, bindings)?;
                let res_2 = self.check_condition_helper(*cond_2, bindings)?;
                match (res_1, res_2) {
                    (PraxsmthConstant::Boolean(b1), PraxsmthConstant::Boolean(b2)) => {
                        Ok(PraxsmthConstant::Boolean(b1 || b2))
                    }
                    (a, b) => bail!(
                        "Or conditions must evaluate to boolean, got {:?} and {:?}",
                        a,
                        b
                    ),
                }
            }

            Condition::Is(cond_1, cond_2) => {
                let res_1 = self.check_condition_helper(*cond_1, bindings)?;
                let res_2 = self.check_condition_helper(*cond_2, bindings)?;
                Ok(PraxsmthConstant::Boolean(res_1 == res_2))
            }

            Condition::Not(cond) => {
                let res = self.check_condition_helper(*cond, bindings)?;
                match res {
                    PraxsmthConstant::Boolean(b) => Ok(PraxsmthConstant::Boolean(!b)),
                    other => bail!("Not condition must evaluate to boolean, got {:?}", other),
                }
            }
        }
    }

    pub fn check_condition(&self, condition: Condition, bindings: &Bindings) -> Result<bool> {
        match self.check_condition_helper(condition, bindings)? {
            PraxsmthConstant::Boolean(b) => Ok(b),
            other => bail!("condition must evaluate to boolean, got {:?}", other),
        }
    }

    fn process_print(
        &self,
        speaker: Option<&str>,
        string: &str,
        bindings: &Bindings,
    ) -> Result<Dialog> {
        Ok(Dialog {
            speaker: speaker.map(|s| s.to_string()),
            line: self.format_string(string, bindings).with_context(|| {
                format!(
                    "formatting string for print outcome with speaker {:?}: {}",
                    speaker, string
                )
            })?,
        })
    }

    fn process_delete(&mut self, sentence: &Sentence, bindings: &Bindings) -> Result<()> {
        let (query, args) = self
            .build_query(sentence, bindings)
            .with_context(|| format!("processing delete outcome {:?}", sentence))?;
        if !args.is_empty() {
            bail!("extra parameters in delete outcome {:?}", sentence);
        }

        let (handle, _) = self
            .lookup_relation(query)
            .with_context(|| format!("relation not found in delete outcome {:?}", sentence))?;
        self.remove_relation(handle)
            .with_context(|| format!("removing relation in delete outcome {:?}", sentence))
    }

    fn process_update(
        &mut self,
        sentence: &Sentence,
        value: &PraxsmthValue,
        bindings: &Bindings,
    ) -> Result<()> {
        let (query, args) = self
            .build_query(sentence, bindings)
            .with_context(|| format!("processing set outcome {:?}", sentence))?;
        if args.len() != 1 {
            // TODO: Support bracket syntax so this is actually usable!!!
            // (Currently you can't initialize anything with >1 fields)
            // Just needs to be added to parser and the logic flow here,
            // the update functions should already support it.
            bail!(
                "exactly one parameter must be specified in set outcome {:?}, got {}",
                sentence,
                args.len()
            );
        }
        let arg_name = &args[0];

        let (handle, _) = self
            .lookup_relation(query)
            .with_context(|| format!("relation not found in set outcome {:?}", sentence))?;

        let constant_value = match value {
            PraxsmthValue::Number(n) => PraxsmthConstant::Number(*n),
            PraxsmthValue::Boolean(b) => PraxsmthConstant::Boolean(*b),
            PraxsmthValue::Variant(v) => PraxsmthConstant::Variant(v.clone()),
            PraxsmthValue::String(s) => PraxsmthConstant::String(s.clone()),
            PraxsmthValue::Variable(sentence) => self
                .resolve_variable(sentence, bindings)
                .context("resolving variable for set outcome")?,
        };

        self.update_relation(handle, HashMap::from([(arg_name.clone(), constant_value)]))
            .with_context(|| format!("applying set outcome {:?}", sentence))
    }

    fn process_increase(&mut self, _sentence: &Sentence, _amount: i64) -> Result<()> {
        unimplemented!()
    }

    fn process_cycle(&mut self, _sentence: &Sentence, _amount: i64) -> Result<()> {
        unimplemented!()
    }

    pub fn process_outcome(
        &mut self,
        agent_name: &str,
        outcome: &PracticeOutcome,
        bindings: &Bindings,
    ) -> Result<Option<Dialog>> {
        match outcome {
            PracticeOutcome::Broadcast(string) => {
                return Ok(Some(self.process_print(None, string, bindings)?));
            }
            PracticeOutcome::Say(string) => {
                return Ok(Some(self.process_print(
                    Some(agent_name),
                    string,
                    bindings,
                )?));
            }
            PracticeOutcome::Activate(agent_id) => {
                self.set_agent_active(&bindings.get_or_same(agent_id), true)
            }
            PracticeOutcome::Deactivate(agent_id) => {
                self.set_agent_active(&bindings.get_or_same(agent_id), false)
            }
            PracticeOutcome::Delete(sentence) => self.process_delete(sentence, bindings),
            PracticeOutcome::Set(declaration) => {
                self.process_declaration(declaration, bindings).map(|_| ())
            }
            PracticeOutcome::Update(sentence, value) => {
                self.process_update(sentence, value, bindings)
            }
            PracticeOutcome::Increase(sentence, amount) => self.process_increase(sentence, *amount),
            PracticeOutcome::Cycle(sentence, amount) => self.process_cycle(sentence, *amount),
        }?;
        Ok(None)
    }

    pub fn get_available_actions(&self, agent_name: &str) -> Result<Vec<AvailableAction>> {
        let agent = self
            .get_agent(agent_name)
            .with_context(|| format!("agent {} not found", agent_name))?;
        let mut available_actions = Vec::new();

        for edge in agent.edges.iter() {
            match edge {
                AgentToRelation::Practice(handle) => {
                    let relation = self.get_relation(handle.clone()).with_context(|| {
                        format!("relation {:?} not found for practice action", handle)
                    })?;
                    let relation_type = self
                        .type_mapping
                        .get_type(&relation.type_name)
                        .with_context(|| {
                            format!("type {} not found for practice action", relation.type_name)
                        })?;
                    let RelationData::Practice { bindings } = &relation.data else {
                        bail!(
                            "relation {:?} data is not practice (was {:?})",
                            handle,
                            relation.data
                        );
                    };
                    let PraxsmthTypeData::Practice { actions, .. } = &relation_type.data else {
                        bail!(
                            "type {} data is not practice for action lookup",
                            relation.type_name
                        );
                    };
                    'action_loop: for (i, action) in actions.iter().enumerate() {
                        let action_for = Self::resolve_binding_or_same(&action.for_actor, bindings);
                        if action_for != agent_name {
                            continue;
                        }
                        for condition in &action.conditions {
                            if !self
                                .check_condition(condition.clone(), bindings)
                                .with_context(|| {
                                    format!(
                                        "checking conditions for action {} of practice {:?}",
                                        action.name, handle
                                    )
                                })?
                            {
                                continue 'action_loop;
                            }
                        }

                        available_actions.push(AvailableAction {
                            display_name: self.format_string(&action.name, bindings).with_context(
                                || {
                                    format!(
                                        "formatting display name for action {} of practice {:?}",
                                        action.name, handle
                                    )
                                },
                            )?,
                            overall_index: available_actions.len(),
                            practice_handle: handle.clone(),
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
        available_action: &AvailableAction,
    ) -> Result<Vec<Dialog>> {
        let relation = self
            .get_relation(available_action.practice_handle.clone())
            .with_context(|| {
                format!(
                    "relation {:?} not found for available action",
                    available_action.practice_handle
                )
            })?;
        let RelationData::Practice { bindings } = &relation.data else {
            bail!(
                "relation {:?} data is not practice for available action",
                available_action.practice_handle
            );
        };
        let relation_type = self
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
        let actor_name = Self::resolve_binding_or_same(&action.for_actor, &bindings);
        let outcomes = action.outcomes.clone();
        let action_name = action.name.clone();
        let bindings = bindings.clone();

        let mut dialog: Vec<Dialog> = vec![];

        for outcome in &outcomes {
            if let Some(new_dialog) = self
                .process_outcome(&actor_name, outcome, &bindings)
                .with_context(|| format!("processing outcome of action {}", action_name))?
            {
                dialog.push(new_dialog);
            }
        }

        Ok(dialog)
    }
}
