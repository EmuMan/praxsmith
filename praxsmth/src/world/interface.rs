use std::collections::HashMap;

use crate::{
    definitions::{
        PraxsmthConstant, PraxsmthValue, Sentence,
        types::{PracticeCondition, PracticeOutcome},
        world::Declaration,
    },
    world::{Relation, RelationHandle, World},
};

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

impl World {
    /// Parses a sentence into a relation query, returning the query
    /// and any remaining parameters.
    pub fn build_query(
        &self,
        sentence: &Sentence,
    ) -> Result<(RelationQuery, Box<[String]>), &'static str> {
        // TODO: verify relation types on top of agent existence
        match sentence.as_slice() {
            [agent, verb, trait_name, rest @ ..] if verb == "is" => {
                self.get_agent(agent).ok_or("Agent not found")?;
                Ok((
                    RelationQuery::Trait {
                        agent: agent.clone(),
                        trait_name: trait_name.clone(),
                    },
                    rest.into(),
                ))
            }
            [agent, verb, emotion_name, rest @ ..] if verb == "feels" => {
                self.get_agent(agent).ok_or("Agent not found")?;
                Ok((
                    RelationQuery::Emotion {
                        agent: agent.clone(),
                        emotion_name: emotion_name.clone(),
                    },
                    rest.into(),
                ))
            }
            [agent_1, relation_type, agent_2, rest @ ..] => {
                self.get_agent(agent_1).ok_or("Agent 1 not found")?;
                self.get_agent(agent_2).ok_or("Agent 2 not found")?;
                Ok((
                    RelationQuery::Binary {
                        agent_1: agent_1.clone(),
                        agent_2: agent_2.clone(),
                        type_name: relation_type.clone(),
                    },
                    rest.into(),
                ))
            }
            [practice, practice_type, rest @ ..] if practice == "practice" => {
                // split up participants into known agents and unknown parameters
                let mut participants = Vec::new();
                for part in rest {
                    if let Some(_) = self.get_agent(part) {
                        participants.push(part.clone());
                    } else {
                        break;
                    }
                }
                let participants_len = participants.len();
                Ok((
                    RelationQuery::Practice {
                        participants: participants,
                        type_name: practice_type.clone(),
                    },
                    rest[participants_len..].into(),
                ))
            }
            _ => Err("Could not parse sentence into a relation query"),
        }
    }

    pub fn process_declaration(
        &mut self,
        declaration: Declaration,
    ) -> Result<RelationHandle, String> {
        let (query, params) = self.build_query(&declaration.sentence)?;
        // TODO: relations with one parameter should be initializable this way!
        if !params.is_empty() {
            return Err("Extra parameters in declaration".into());
        }
        match query {
            RelationQuery::Trait { agent, trait_name } => {
                self.add_trait(&agent, &trait_name, declaration.fields)
            }
            RelationQuery::Emotion {
                agent,
                emotion_name,
            } => self.add_emotion(&agent, &emotion_name, declaration.fields),
            RelationQuery::Binary {
                agent_1,
                agent_2,
                type_name,
            } => self.add_binary_relation(&agent_1, &agent_2, &type_name, declaration.fields),
            RelationQuery::Practice {
                participants,
                type_name,
            } => self.add_practice(participants, &type_name, declaration.fields),
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

    pub fn resolve_variable(&self, sentence: &Sentence) -> Result<PraxsmthConstant, String> {
        let (query, params) = self.build_query(&sentence)?;

        let Some((_, relation)) = self.lookup_relation(query) else {
            // No relation found, so this evaluates to false
            // TODO: when better error handling is added to lookups, this should be updated as well.
            return Ok(PraxsmthConstant::Boolean(false));
        };

        if params.is_empty() {
            // No parameters specified but there is a relationship, so return true
            return Ok(PraxsmthConstant::Boolean(true));
        }

        if params.len() > 1 {
            return Err("Too many parameters specified in variable".into());
        }

        // A parameter was specified, so pull it from the relation's fields
        let param_name = &params[0];
        if let Some(constant) = relation.fields.get(param_name) {
            Ok(constant.clone())
        } else {
            Err(format!("Parameter '{}' not found in relation", param_name))
        }
    }

    fn check_condition_helper(
        &self,
        condition: PracticeCondition,
    ) -> Result<PraxsmthConstant, String> {
        match condition {
            PracticeCondition::Value(value) => match value {
                PraxsmthValue::Number(n) => Ok(PraxsmthConstant::Number(n)),
                PraxsmthValue::Boolean(b) => Ok(PraxsmthConstant::Boolean(b)),
                PraxsmthValue::Variant(v) => Ok(PraxsmthConstant::Variant(v)),
                PraxsmthValue::String(s) => Ok(PraxsmthConstant::String(s)),
                PraxsmthValue::Variable(sentence) => self.resolve_variable(&sentence),
            },

            PracticeCondition::And(cond_1, cond_2) => {
                let res_1 = self.check_condition_helper(*cond_1)?;
                let res_2 = self.check_condition_helper(*cond_2)?;
                match (res_1, res_2) {
                    (PraxsmthConstant::Boolean(b1), PraxsmthConstant::Boolean(b2)) => {
                        Ok(PraxsmthConstant::Boolean(b1 && b2))
                    }
                    // TODO: better error description
                    _ => Err("Conditions must evaluate to boolean".into()),
                }
            }

            PracticeCondition::Or(cond_1, cond_2) => {
                let res_1 = self.check_condition_helper(*cond_1)?;
                let res_2 = self.check_condition_helper(*cond_2)?;
                match (res_1, res_2) {
                    (PraxsmthConstant::Boolean(b1), PraxsmthConstant::Boolean(b2)) => {
                        Ok(PraxsmthConstant::Boolean(b1 || b2))
                    }
                    _ => Err("Conditions must evaluate to boolean".into()),
                }
            }

            PracticeCondition::Is(cond_1, cond_2) => {
                let res_1 = self.check_condition_helper(*cond_1)?;
                let res_2 = self.check_condition_helper(*cond_2)?;
                Ok(PraxsmthConstant::Boolean(res_1 == res_2))
            }

            PracticeCondition::Not(cond) => {
                let res = self.check_condition_helper(*cond)?;
                match res {
                    PraxsmthConstant::Boolean(b) => Ok(PraxsmthConstant::Boolean(!b)),
                    _ => Err("Condition must evaluate to boolean".into()),
                }
            }
        }
    }

    pub fn check_condition(&self, condition: PracticeCondition) -> Result<bool, String> {
        match self.check_condition_helper(condition)? {
            PraxsmthConstant::Boolean(b) => Ok(b),
            _ => Err("Condition must evaluate to boolean".into()),
        }
    }

    fn process_delete(&mut self, sentence: &Sentence) -> Result<(), String> {
        let (query, params) = self
            .build_query(sentence)
            .expect("Invalid sentence in delete outcome");
        if !params.is_empty() {
            panic!("Extra parameters in delete outcome");
        }

        match self.lookup_relation(query) {
            Some((handle, _)) => self.remove_relation(handle),
            None => return Err("Relation not found in delete outcome".into()),
        }
    }

    fn process_set(&mut self, sentence: &Sentence, value: &PraxsmthValue) -> Result<(), String> {
        let (query, params) = self
            .build_query(sentence)
            .expect("Invalid sentence in set outcome");
        if params.len() != 1 {
            // TODO: Support bracket syntax so this is actually usable!!!
            // Just needs to be added to parser and the logic flow here,
            // the update functions should already support it.
            panic!("Expected exactly one parameter in set outcome");
        }
        let param_name = &params[0];

        let (handle, _) = self
            .lookup_relation(query)
            .ok_or("Relation not found in set outcome")?;

        let constant_value = match value {
            PraxsmthValue::Number(n) => PraxsmthConstant::Number(*n),
            PraxsmthValue::Boolean(b) => PraxsmthConstant::Boolean(*b),
            PraxsmthValue::Variant(v) => PraxsmthConstant::Variant(v.clone()),
            PraxsmthValue::String(s) => PraxsmthConstant::String(s.clone()),
            PraxsmthValue::Variable(sentence) => self.resolve_variable(sentence)?,
        };

        self.update_relation(
            handle,
            HashMap::from([(param_name.clone(), constant_value)]),
        )
    }

    fn process_increase(&mut self, sentence: &Sentence, amount: i64) -> Result<(), String> {
        unimplemented!()
    }

    fn process_cycle(&mut self, sentence: &Sentence, amount: i64) -> Result<(), String> {
        unimplemented!()
    }

    pub fn process_outcome(&mut self, outcome: &PracticeOutcome) -> Result<(), String> {
        match outcome {
            PracticeOutcome::Print(string) => {
                println!("{}", string);
                Ok(())
            }
            PracticeOutcome::Delete(sentence) => self.process_delete(sentence),
            PracticeOutcome::Set(sentence, value) => self.process_set(sentence, value),
            PracticeOutcome::Increase(sentence, amount) => self.process_increase(sentence, *amount),
            PracticeOutcome::Cycle(sentence, amount) => self.process_cycle(sentence, *amount),
        }
    }
}
