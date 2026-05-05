use std::collections::HashMap;

use crate::{
    definitions::{
        PraxsmthConstant, PraxsmthValue, Sentence,
        types::{PracticeCondition, PracticeOutcome, PraxsmthTypeData},
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
        agent: String,
        trait_name: String,
    },
    Emotion {
        agent: String,
        emotion_name: String,
    },
    Binary {
        agent_1: String,
        agent_2: String,
        type_name: String,
    },
    Practice {
        participants: Vec<String>,
        type_name: String,
    },
}

impl RelationQuery {
    pub fn apply_bindings(&self, bindings: &Bindings) -> RelationQuery {
        match self {
            RelationQuery::Trait { agent, trait_name } => RelationQuery::Trait {
                agent: bindings.get(agent).cloned().unwrap_or(agent.clone()),
                trait_name: trait_name.clone(),
            },
            RelationQuery::Emotion {
                agent,
                emotion_name,
            } => RelationQuery::Emotion {
                agent: bindings.get(agent).cloned().unwrap_or(agent.clone()),
                emotion_name: emotion_name.clone(),
            },
            RelationQuery::Binary {
                agent_1,
                agent_2,
                type_name,
            } => RelationQuery::Binary {
                agent_1: bindings.get(agent_1).cloned().unwrap_or(agent_1.clone()),
                agent_2: bindings.get(agent_2).cloned().unwrap_or(agent_2.clone()),
                type_name: type_name.clone(),
            },
            RelationQuery::Practice {
                participants,
                type_name,
            } => RelationQuery::Practice {
                participants: participants
                    .iter()
                    .map(|p| bindings.get(p).cloned().unwrap_or(p.clone()))
                    .collect(),
                type_name: type_name.clone(),
            },
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
    ) -> Result<(RelationQuery, Box<[String]>), &'static str> {
        match sentence.as_slice() {
            [agent, verb, trait_name, rest @ ..] if verb == "is" => {
                let Some(relation_type) = self.type_mapping.get_type(trait_name) else {
                    return Err("Relation type not found");
                };
                let PraxsmthTypeData::Trait { .. } = &relation_type.data else {
                    return Err("Relation type is not a trait");
                };
                Ok((
                    RelationQuery::Trait {
                        agent: agent.clone(),
                        trait_name: trait_name.clone(),
                    },
                    rest.into(),
                ))
            }
            [agent, verb, emotion_name, rest @ ..] if verb == "feels" => {
                let Some(relation_type) = self.type_mapping.get_type(emotion_name) else {
                    return Err("Relation type not found");
                };
                let PraxsmthTypeData::Emotion { .. } = &relation_type.data else {
                    return Err("Relation type is not an emotion");
                };
                Ok((
                    RelationQuery::Emotion {
                        agent: agent.clone(),
                        emotion_name: emotion_name.clone(),
                    },
                    rest.into(),
                ))
            }
            [practice, practice_name, rest @ ..] if practice == "practice" => {
                // for this one we have to look up the type to see how many parameters it takes
                let Some(relation_type) = self.type_mapping.get_type(practice_name) else {
                    return Err("Relation type not found");
                };
                let PraxsmthTypeData::Practice { params, .. } = &relation_type.data else {
                    return Err("Relation type is not a practice");
                };
                let participants_count = params.len();
                if rest.len() < participants_count {
                    return Err("Not enough parameters for practice relation");
                }
                let participants = rest[..participants_count].iter().cloned().collect();
                Ok((
                    RelationQuery::Practice {
                        participants: participants,
                        type_name: practice_name.clone(),
                    },
                    rest[participants_count..].into(),
                ))
            }
            [agent_1, relation_name, agent_2, rest @ ..] => {
                let Some(relation_type) = self.type_mapping.get_type(relation_name) else {
                    return Err("Relation type not found");
                };
                match &relation_type.data {
                    PraxsmthTypeData::Directional { .. } => {}
                    PraxsmthTypeData::Reciprocal { .. } => {}
                    PraxsmthTypeData::Evaluation { .. } => {}
                    _ => return Err("Relation type is not a binary relation"),
                }
                Ok((
                    RelationQuery::Binary {
                        agent_1: agent_1.clone(),
                        agent_2: agent_2.clone(),
                        type_name: relation_name.clone(),
                    },
                    rest.into(),
                ))
            }
            _ => Err("Could not parse sentence into a relation query"),
        }
    }

    pub fn process_declaration(
        &mut self,
        declaration: &Declaration,
    ) -> Result<RelationHandle, String> {
        // This query does not have any bindings because declarations are only
        // used to declare things at a high level.
        let (query, args) = self.build_query(&declaration.sentence)?;
        // TODO: relations with one parameter should be initializable this way!
        if !args.is_empty() {
            return Err("Extra parameters in declaration".into());
        }
        match query {
            RelationQuery::Trait { agent, trait_name } => {
                self.add_trait(&agent, &trait_name, declaration.fields.clone())
            }
            RelationQuery::Emotion {
                agent,
                emotion_name,
            } => self.add_emotion(&agent, &emotion_name, declaration.fields.clone()),
            RelationQuery::Binary {
                agent_1,
                agent_2,
                type_name,
            } => {
                self.add_binary_relation(&agent_1, &agent_2, &type_name, declaration.fields.clone())
            }
            RelationQuery::Practice {
                participants,
                type_name,
            } => self.add_practice(participants, &type_name, declaration.fields.clone()),
        }
    }

    pub fn lookup_relation(&self, query: RelationQuery) -> Option<(RelationHandle, &Relation)> {
        match query {
            RelationQuery::Trait { agent, trait_name } => self.get_trait(&agent, &trait_name),
            RelationQuery::Emotion {
                agent,
                emotion_name,
            } => self.get_emotion(&agent, &emotion_name),
            RelationQuery::Binary {
                agent_1,
                agent_2,
                type_name,
            } => self.get_binary_relation(&agent_1, &agent_2, &type_name),
            RelationQuery::Practice {
                participants,
                type_name,
            } => self.get_practice(participants, &type_name),
        }
    }

    pub fn resolve_variable(
        &self,
        sentence: &Sentence,
        bindings: &Bindings,
    ) -> Result<PraxsmthConstant, String> {
        let (query, args) = self.build_query(&sentence)?;
        let query = query.apply_bindings(bindings);

        let Some((_, relation)) = self.lookup_relation(query) else {
            // No relation found, so this evaluates to false
            // TODO: when better error handling is added to lookups, this should be updated as well.
            return Ok(PraxsmthConstant::Boolean(false));
        };

        if args.is_empty() {
            // No parameters specified but there is a relationship, so return true
            return Ok(PraxsmthConstant::Boolean(true));
        }

        if args.len() > 1 {
            return Err("Too many parameters specified in variable".into());
        }

        // An argument was specified, so pull it from the relation's fields
        let arg_name = &args[0];
        if let Some(constant) = relation.fields.get(arg_name) {
            Ok(constant.clone())
        } else {
            Err(format!("Parameter '{}' not found in relation", arg_name))
        }
    }

    fn check_condition_helper(
        &self,
        condition: PracticeCondition,
        bindings: &Bindings,
    ) -> Result<PraxsmthConstant, String> {
        match condition {
            PracticeCondition::Value(value) => match value {
                PraxsmthValue::Number(n) => Ok(PraxsmthConstant::Number(n)),
                PraxsmthValue::Boolean(b) => Ok(PraxsmthConstant::Boolean(b)),
                PraxsmthValue::Variant(v) => Ok(PraxsmthConstant::Variant(v)),
                PraxsmthValue::String(s) => Ok(PraxsmthConstant::String(s)),
                PraxsmthValue::Variable(sentence) => self.resolve_variable(&sentence, bindings),
            },

            PracticeCondition::And(cond_1, cond_2) => {
                let res_1 = self.check_condition_helper(*cond_1, bindings)?;
                let res_2 = self.check_condition_helper(*cond_2, bindings)?;
                match (res_1, res_2) {
                    (PraxsmthConstant::Boolean(b1), PraxsmthConstant::Boolean(b2)) => {
                        Ok(PraxsmthConstant::Boolean(b1 && b2))
                    }
                    // TODO: better error description
                    _ => Err("Conditions must evaluate to boolean".into()),
                }
            }

            PracticeCondition::Or(cond_1, cond_2) => {
                let res_1 = self.check_condition_helper(*cond_1, bindings)?;
                let res_2 = self.check_condition_helper(*cond_2, bindings)?;
                match (res_1, res_2) {
                    (PraxsmthConstant::Boolean(b1), PraxsmthConstant::Boolean(b2)) => {
                        Ok(PraxsmthConstant::Boolean(b1 || b2))
                    }
                    _ => Err("Conditions must evaluate to boolean".into()),
                }
            }

            PracticeCondition::Is(cond_1, cond_2) => {
                let res_1 = self.check_condition_helper(*cond_1, bindings)?;
                let res_2 = self.check_condition_helper(*cond_2, bindings)?;
                Ok(PraxsmthConstant::Boolean(res_1 == res_2))
            }

            PracticeCondition::Not(cond) => {
                let res = self.check_condition_helper(*cond, bindings)?;
                match res {
                    PraxsmthConstant::Boolean(b) => Ok(PraxsmthConstant::Boolean(!b)),
                    _ => Err("Condition must evaluate to boolean".into()),
                }
            }
        }
    }

    pub fn check_condition(
        &self,
        condition: PracticeCondition,
        bindings: &Bindings,
    ) -> Result<bool, String> {
        match self.check_condition_helper(condition, bindings)? {
            PraxsmthConstant::Boolean(b) => Ok(b),
            _ => Err("Condition must evaluate to boolean".into()),
        }
    }

    fn process_delete(&mut self, sentence: &Sentence, bindings: &Bindings) -> Result<(), String> {
        let (query, args) = self.build_query(sentence)?;
        let query = query.apply_bindings(bindings);
        if !args.is_empty() {
            return Err("Extra parameters in delete outcome".into());
        }

        match self.lookup_relation(query) {
            Some((handle, _)) => self.remove_relation(handle),
            None => return Err("Relation not found in delete outcome".into()),
        }
    }

    fn process_set(
        &mut self,
        sentence: &Sentence,
        value: &PraxsmthValue,
        bindings: &Bindings,
    ) -> Result<(), String> {
        let (query, args) = self.build_query(sentence)?;
        let query = query.apply_bindings(bindings);
        if args.len() != 1 {
            // TODO: Support bracket syntax so this is actually usable!!!
            // (Currently you can't initialize anything with >1 fields)
            // Just needs to be added to parser and the logic flow here,
            // the update functions should already support it.
            return Err("Exactly one parameter must be specified in set outcome".into());
        }
        let arg_name = &args[0];

        let (handle, _) = self
            .lookup_relation(query)
            .ok_or("Relation not found in set outcome")?;

        let constant_value = match value {
            PraxsmthValue::Number(n) => PraxsmthConstant::Number(*n),
            PraxsmthValue::Boolean(b) => PraxsmthConstant::Boolean(*b),
            PraxsmthValue::Variant(v) => PraxsmthConstant::Variant(v.clone()),
            PraxsmthValue::String(s) => PraxsmthConstant::String(s.clone()),
            PraxsmthValue::Variable(sentence) => self.resolve_variable(sentence, bindings)?,
        };

        self.update_relation(handle, HashMap::from([(arg_name.clone(), constant_value)]))
    }

    fn process_increase(&mut self, sentence: &Sentence, amount: i64) -> Result<(), String> {
        unimplemented!()
    }

    fn process_cycle(&mut self, sentence: &Sentence, amount: i64) -> Result<(), String> {
        unimplemented!()
    }

    pub fn process_outcome(
        &mut self,
        agent_name: &str,
        outcome: &PracticeOutcome,
        bindings: &Bindings,
    ) -> Result<Option<Dialog>, String> {
        match outcome {
            PracticeOutcome::Print(string) => {
                return Ok(Some(Dialog {
                    speaker: Some(agent_name.into()),
                    line: string.clone(),
                }));
            }
            PracticeOutcome::Delete(sentence) => self.process_delete(sentence, bindings),
            PracticeOutcome::Set(sentence, value) => self.process_set(sentence, value, bindings),
            PracticeOutcome::Increase(sentence, amount) => self.process_increase(sentence, *amount),
            PracticeOutcome::Cycle(sentence, amount) => self.process_cycle(sentence, *amount),
        }?;
        Ok(None)
    }

    pub fn get_available_actions(&self, agent_name: &str) -> Result<Vec<AvailableAction>, String> {
        let agent = self.get_agent(agent_name).ok_or("Agent not found")?;
        let mut available_actions = Vec::new();

        for edge in agent.edges.iter() {
            match edge {
                AgentToRelation::Practice(handle) => {
                    let relation = self
                        .get_relation(handle.clone())
                        .ok_or("Relation not found for practice action")?;
                    let relation_type = self
                        .type_mapping
                        .get_type(&relation.type_name)
                        .ok_or("Type not found for practice action")?;
                    let RelationData::Practice { bindings } = &relation.data else {
                        return Err("Relation data for practice action is not practice".into());
                    };
                    let PraxsmthTypeData::Practice { actions, .. } = &relation_type.data else {
                        return Err("Relation type for practice action is not a practice".into());
                    };
                    'action_loop: for (i, action) in actions.iter().enumerate() {
                        for condition in &action.conditions {
                            if !self.check_condition(condition.clone(), bindings)? {
                                continue 'action_loop;
                            }
                        }
                        available_actions.push(AvailableAction {
                            display_name: action.name.clone(),
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
    ) -> Result<Vec<Dialog>, String> {
        let Some(relation) = self.get_relation(available_action.practice_handle.clone()) else {
            return Err("Relation not found for available action".into());
        };
        let RelationData::Practice { bindings } = &relation.data else {
            return Err("Relation data for available action is not practice".into());
        };
        let relation_type = self
            .type_mapping
            .get_type(&relation.type_name)
            .ok_or("Type not found for available action")?;
        let PraxsmthTypeData::Practice { actions, .. } = &relation_type.data else {
            return Err("Relation type for available action is not a practice".into());
        };
        let action = actions
            .get(available_action.index_within_practice)
            .ok_or("Action index out of bounds for available action")?;

        // "no meaningful cost concern here"... says Claude. Not convinced.
        // TODO: Fix this a better way.
        let actor_name = action.for_actor.clone();
        let outcomes = action.outcomes.clone();
        let bindings = bindings.clone();

        let mut dialog: Vec<Dialog> = vec![];

        for outcome in &outcomes {
            if let Some(new_dialog) = self.process_outcome(&actor_name, outcome, &bindings)? {
                dialog.push(new_dialog);
            }
        }

        Ok(dialog)
    }
}
