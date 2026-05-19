use std::collections::{HashMap, HashSet};

use anyhow::{Context, Result, bail};

use crate::{
    anyhow_ext::ResultOptionExt,
    definitions::{
        PraxsmthConstant, PraxsmthValue, Sentence,
        types::{Condition, Expression, PracticeOutcome, PraxsmthTypeData, ResolutionMethod},
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

#[derive(Debug, Clone)]
pub struct Dialog {
    pub speaker: Option<String>,
    pub line: String,
}

/// The simulation component of the world, responsible for processing
/// declarations, evaluating variables, and generally doing the work of turning
/// the static world state into a dynamic, interactive simulation.
///
/// The world state and simulation are tied together through a `PraxsmthApi`.
#[derive(Debug, Clone)]
pub struct Simulation {
    pub dialog_history: Vec<Dialog>,
}

impl Default for Simulation {
    fn default() -> Self {
        Self::new()
    }
}

impl Simulation {
    pub fn new() -> Self {
        Self {
            dialog_history: Vec::new(),
        }
    }

    /// Parses a sentence into a relation query, returning the query and any
    /// remaining arguments. Verifies the type of the relation and the number
    /// of parameters, but does not verify the agents involved.
    ///
    /// If the sentence is just ["self"], recurses on `bindings.self_id` if it
    /// exists.
    ///
    /// Returns the relation query, plus any extra arguments at the end of the
    /// sentence.
    pub fn build_query(
        &self,
        world: &World,
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
                let (query, _) = self
                    .build_query(world, self_sentence, bindings)
                    .with_context(|| {
                        format!(
                            "parsing sentence starting with 'self' using self context {:?}",
                            self_sentence
                        )
                    })?;
                Ok((query, rest.into()))
            }
            [agent, verb, trait_name, rest @ ..] if verb == "is" => {
                let relation_type = world
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
                let relation_type = world
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

    /// Adds the information contained within a declaration to the world state.
    ///
    /// The sentence within the declaration must match a query. An error will
    /// be raised if there are any free variables within this query.
    pub fn process_declaration(
        &mut self,
        world: &mut World,
        declaration: &Declaration,
        bindings: &Bindings,
    ) -> Result<RelationHandle> {
        let (query, args) = self
            .build_query(world, &declaration.sentence, bindings)
            .with_context(|| format!("processing declaration {:?}", declaration.sentence))?;
        // TODO: relations with one parameter should be initializable this way!
        if !args.is_empty() {
            bail!("extra parameters in declaration {:?}", declaration.sentence);
        }
        match query {
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
    pub fn lookup_relation<'a>(
        &self,
        world: &'a World,
        query: RelationQuery,
    ) -> Result<(&'a AgentToRelation, &'a Relation)> {
        match query {
            RelationQuery::Trait { agent, trait_name } => {
                let agent_lit = agent.as_literal()?;
                world
                    .get_trait(agent_lit, &trait_name)
                    .require_with_context(|| {
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
                    .require_with_context(|| {
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
                world.get_binary_relation(agent_1_lit, agent_2_lit, &type_name).require_with_context(|| {
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
                    .require_with_context(|| {
                        format!(
                            "could not find practice with participants: {:?}, practice name: {}",
                            participants, type_name
                        )
                    })
            }
        }
    }

    /// Turns a variable (sentence defining a world query) into a constant
    /// value. If the relation specified within the variable does not exist,
    /// evaluates to `PraxsmthConstant::Boolean(false)`. If there is no field
    /// specified but the relationship does exist, evaluates to
    /// `PraxsmthConstant::Boolean(true)`.
    ///
    /// Returns an error if there are any free variables in the sentence. All
    /// variables must be defined within `bindings`.
    pub fn resolve_variable(
        &self,
        world: &World,
        sentence: &Sentence,
        bindings: &Bindings,
    ) -> Result<PraxsmthConstant> {
        let (query, args) = self
            .build_query(world, &sentence, bindings)
            .with_context(|| format!("resolving variable {:?}", sentence))?;

        // TODO: propagate relation not found, but NOT free variable errors!!!!
        let Ok((_, relation)) = self.lookup_relation(world, query) else {
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

    /// Finds all agents with the specified trait. Does not verify that the
    /// trait is the correct type. Used as a helper for
    /// `World::find_all_valid_bindings(...)`.
    fn find_agents_with_trait<'a>(&self, world: &'a World, trait_id: &str) -> Vec<&'a str> {
        let mut agents_with_trait = Vec::new();
        for agent in world.agents.values() {
            match world.get_trait(agent.name.as_str(), trait_id) {
                Ok(Some(_)) => agents_with_trait.push(agent.name.as_str()),
                _ => {}
            }
        }
        agents_with_trait
    }

    /// Finds all agents with the specified emotion. Does not verify that the
    /// emotion is the correct type. Used as a helper for
    /// `World::find_all_valid_bindings(...)`.
    fn find_agents_with_emotion<'a>(&self, world: &'a World, emotion_id: &str) -> Vec<&'a str> {
        let mut agents_with_emotion = Vec::new();
        for agent in world.agents.values() {
            match world.get_emotion(agent.name.as_str(), emotion_id) {
                Ok(Some(_)) => agents_with_emotion.push(agent.name.as_str()),
                _ => {}
            }
        }
        agents_with_emotion
    }

    /// Finds all agents with the specified binary relation. Does not verify
    /// that the binary relation is the correct type. Used as a helper for
    /// `World::find_all_valid_bindings(...)`.
    fn find_agents_with_binary_relation<'a>(
        &self,
        world: &'a World,
        type_id: &str,
    ) -> Vec<(&'a str, &'a str)> {
        // Efficiency out the window, just go through every agent pair
        // TODO: better?
        let mut agent_pairs = Vec::new();
        // Some of these are unordered, so keep track of handles we've already
        // seen to avoid duplicates.
        let mut seen_handles = HashSet::new();
        for agent_1_id in world.agents.keys() {
            for agent_2_id in world.agents.keys() {
                match world.get_binary_relation(agent_1_id, agent_2_id, type_id) {
                    Ok(Some((edge, _))) => {
                        if !seen_handles.contains(&edge.relation_handle) {
                            agent_pairs.push((agent_1_id.as_str(), agent_2_id.as_str()));
                            seen_handles.insert(edge.relation_handle.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
        agent_pairs
    }

    /// Finds all agents that sit on the second end of the specified binary
    /// relation. Does not verify that the binary relation is the correct type.
    /// Used as a helper for `World::find_all_valid_bindings(...)`.
    fn find_secondary_agents_for_binary_relation<'a>(
        &self,
        world: &'a World,
        type_id: &str,
        agent_1_id: &str,
    ) -> Vec<&'a str> {
        let mut secondary_agents = Vec::new();
        for agent_2_id in world.agents.keys() {
            match world.get_binary_relation(agent_1_id, agent_2_id, type_id) {
                Ok(Some(_)) => {
                    secondary_agents.push(agent_2_id.as_str());
                }
                _ => {}
            }
        }
        secondary_agents
    }

    /// Finds all agents that sit on the first end of the specified binary
    /// relation. Does not verify that the binary relation is the correct type.
    /// Used as a helper for `World::find_all_valid_bindings(...)`.
    fn find_primary_agents_for_binary_relation<'a>(
        &self,
        world: &'a World,
        type_id: &str,
        agent_2_id: &str,
    ) -> Vec<&'a str> {
        let mut primary_agents = Vec::new();
        for agent_1_id in world.agents.keys() {
            match world.get_binary_relation(agent_1_id, agent_2_id, type_id) {
                Ok(Some(_)) => {
                    primary_agents.push(agent_1_id.as_str());
                }
                _ => {}
            }
        }
        primary_agents
    }

    /// Finds all agents that participate in the specified practice, given the
    /// constraint of the participants specified in the arguments. Does not
    /// verify that the practice is the correct type. Used as a helper for
    /// `World::find_all_valid_bindings(...)`.
    fn find_agents_with_practice<'a>(
        &self,
        world: &'a World,
        type_id: &str,
        constraints: &[Option<&str>],
    ) -> Vec<&'a Vec<String>> {
        let mut participant_sets = Vec::new();
        world.iter_relations().filter(|(_, relation)| {
            // type name and type should match
            matches!(
                world.type_mapping.get_type(&relation.type_name),
                Some(t) if t.name == type_id && matches!(t.data, PraxsmthTypeData::Practice { .. })
            )
        })
        .filter_map(|(_, relation)| {
            // participant count should match, and also extract participants
            match &relation.data {
                RelationData::Practice { agents, .. } => {
                    // participant count should match
                    if agents.len() != constraints.len() {
                        return None;
                    }
                    // all specified participants should match
                    if agents.iter().zip(constraints.iter()).all(|(agent, constraint)| {
                        constraint.map_or(true, |p| p == *agent)
                    }) {
                        Some(agents)
                    } else {
                        None
                    }
                }
                _ => None,
            }
        })
        .for_each(|agent_set| participant_sets.push(agent_set));

        participant_sets
    }

    /// Finds all extensions of `bindings` that allow for the valid processing
    /// of `sentence`. Valid processing means that the relationship exists and
    /// can be processed normally. Technically, relationships that are not
    /// found still evaluate to `PraxsmthConstant::Boolean(false)`, but
    /// including nonexistent relationships here would make this operation
    /// trivial and useless. This is used as a helper function for variables
    /// found within `World::solve_for_free_vars`.
    ///
    /// If all specified actors within `sentence` are already bound, this will
    /// return a list with a single value, which will be a clone of `bindings`.
    /// No extra checks will be performed to make sure that these agents have
    /// the specified relations, as that is intended to be done later down the
    /// line.
    ///
    /// If there are free symbols, all possible assignments of these free
    /// symbols, given the existing assignments defined in `bindings`, are
    /// effectively enumerated. Any valid set of assignments is returned with
    /// the vector.
    pub fn find_all_valid_bindings(
        &self,
        world: &World,
        sentence: &Sentence,
        bindings: &Bindings,
    ) -> Result<Vec<Bindings>> {
        let (query, _) = self
            .build_query(world, &sentence, bindings)
            .with_context(|| format!("finding valid bindings for sentence {:?}", sentence))?;
        match &query {
            RelationQuery::Trait { agent, trait_name } => match agent {
                AgentRef::Literal(_) => Ok(vec![bindings.clone()]),
                AgentRef::Free(specifier) => Ok(self
                    .find_agents_with_trait(world, trait_name)
                    .into_iter()
                    .map(|agent_name| {
                        bindings.with([(specifier.clone(), agent_name.to_string())].into())
                    })
                    .collect()),
            },
            RelationQuery::Emotion {
                agent,
                emotion_name,
            } => match agent {
                AgentRef::Literal(_) => Ok(vec![bindings.clone()]),
                AgentRef::Free(specifier) => Ok(self
                    .find_agents_with_emotion(world, emotion_name)
                    .into_iter()
                    .map(|agent_name| {
                        bindings.with([(specifier.clone(), agent_name.to_string())].into())
                    })
                    .collect()),
            },
            RelationQuery::Binary {
                agent_1,
                agent_2,
                type_name,
            } => match (agent_1, agent_2) {
                (AgentRef::Literal(_), AgentRef::Literal(_)) => Ok(vec![bindings.clone()]),
                (AgentRef::Free(specifier), AgentRef::Literal(agent_2_name)) => Ok(self
                    .find_primary_agents_for_binary_relation(world, type_name, agent_2_name)
                    .into_iter()
                    .map(|agent_1_name| {
                        bindings.with([(specifier.clone(), agent_1_name.to_string())].into())
                    })
                    .collect()),
                (AgentRef::Literal(agent_1_name), AgentRef::Free(specifier)) => Ok(self
                    .find_secondary_agents_for_binary_relation(world, type_name, agent_1_name)
                    .into_iter()
                    .map(|agent_2_name| {
                        bindings.with([(specifier.clone(), agent_2_name.to_string())].into())
                    })
                    .collect()),
                (AgentRef::Free(specifier_1), AgentRef::Free(specifier_2)) => Ok(self
                    .find_agents_with_binary_relation(world, type_name)
                    .into_iter()
                    .map(|(agent_1_name, agent_2_name)| {
                        bindings.with(
                            [
                                (specifier_1.clone(), agent_1_name.to_string()),
                                (specifier_2.clone(), agent_2_name.to_string()),
                            ]
                            .into(),
                        )
                    })
                    .collect()),
            },
            RelationQuery::Practice {
                participants,
                type_name,
            } => {
                let constraints = participants
                    .iter()
                    .map(|p| match p {
                        AgentRef::Literal(agent_name) => Some(agent_name.as_str()),
                        AgentRef::Free(_) => None,
                    })
                    .collect::<Vec<Option<&str>>>();
                let participant_sets =
                    self.find_agents_with_practice(world, type_name, &constraints);
                // generate bindings using the participant sets and the free variable specifiers
                let mut result_bindings = Vec::new();
                for result_agents in participant_sets {
                    if result_agents.len() != result_agents.len() {
                        bail!("participant count mismatch for practice {}", type_name);
                    }
                    let mut new_bindings = bindings.clone();
                    for (p, result_agent) in participants.iter().zip(result_agents.iter()) {
                        if let AgentRef::Free(specifier) = p {
                            new_bindings.insert(specifier.clone(), result_agent.clone());
                        }
                    }
                    result_bindings.push(new_bindings);
                }
                Ok(result_bindings)
            }
        }
    }

    /// Calculates for all possible extensions of `bindings` that allow for
    /// valid processings of `expression`.
    ///
    /// If no free variables (variables with free actor components) exist in
    /// the expression tree, this will return a list with a single value, which
    /// will be a clone of `bindings`.
    ///
    /// If there are free variables, all possible assignments of these free
    /// variables, given the existing assignments defined in `bindings`, are
    /// effectively enumerated. Any valid set of assignments is returned with
    /// the vector.
    ///
    /// Note that this does not evaluate the expression, it simply finds
    /// bindings that allow for it to be evaluated. Any one of the returned
    /// binding sets can be used with `World::evaluate_expression(...)` for a
    /// proper evaluation.
    pub fn solve_for_free_vars(
        &self,
        world: &World,
        expression: &Expression,
        bindings: &Bindings,
    ) -> Result<Vec<Bindings>> {
        // This is similar to a wave function collapse algorithm, where all
        // possible bindings are generated for free variables (variables with
        // free actor components), and then these bindings are combined in
        // various ways based on the operations between them.
        match expression {
            Expression::Value(PraxsmthValue::Variable(s)) => {
                // This is the main part where bindings get added.
                self.find_all_valid_bindings(world, s, bindings)
            }
            Expression::Value(_) => {
                // Constant values carry no additional bindings with them; they
                // have already been resolved.
                Ok(vec![])
            }
            Expression::And(x, y) => {
                // Symbols on either side are implied to be bound to the same
                // value. This means that the only bindings that work are ones
                // that are shared between `x` and `y`. This is just a merge!
                let x_bindings = self.solve_for_free_vars(world, x, bindings)?;
                let y_bindings = self.solve_for_free_vars(world, y, bindings)?;
                Ok(x_bindings
                    .iter()
                    .flat_map(|xb| y_bindings.iter().filter_map(|yb| xb.try_merge(yb)))
                    .collect())
            }
            Expression::Or(x, y) => {
                // Interestingly, the same shared bindings principle applies
                // here. The "and" vs. "or" distinction is not important, it's
                // that both sides must have equivalent bindings.
                let x_bindings = self.solve_for_free_vars(world, x, bindings)?;
                let y_bindings = self.solve_for_free_vars(world, y, bindings)?;
                Ok(x_bindings
                    .iter()
                    .flat_map(|xb| y_bindings.iter().filter_map(|yb| xb.try_merge(yb)))
                    .collect())
            }
            Expression::Not(e) => {
                // Similar to the above notes, the only thing that matters is
                // that whatever is inside the "not" must be bound with the
                // same bindings as all other expressions. So the bindings can
                // just be passed through here.
                self.solve_for_free_vars(world, e, bindings)
            }
            Expression::Is(x, y) => {
                // Again, this is just a binary operation. Same as "and" and
                // "or".
                let x_bindings = self.solve_for_free_vars(world, x, bindings)?;
                let y_bindings = self.solve_for_free_vars(world, y, bindings)?;
                Ok(x_bindings
                    .iter()
                    .flat_map(|xb| y_bindings.iter().filter_map(|yb| xb.try_merge(yb)))
                    .collect())
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
        expression: Expression,
        bindings: &Bindings,
    ) -> Result<PraxsmthConstant> {
        match expression {
            Expression::Value(value) => match value {
                PraxsmthValue::Number(n) => Ok(PraxsmthConstant::Number(n)),
                PraxsmthValue::Boolean(b) => Ok(PraxsmthConstant::Boolean(b)),
                PraxsmthValue::Variant(v) => Ok(PraxsmthConstant::Variant(v)),
                PraxsmthValue::String(s) => Ok(PraxsmthConstant::String(s)),
                PraxsmthValue::Variable(sentence) => {
                    self.resolve_variable(world, &sentence, bindings)
                }
            },

            Expression::And(x, y) => {
                let x = self.evaluate_expression(world, *x, bindings)?;
                let y = self.evaluate_expression(world, *y, bindings)?;
                match (x, y) {
                    (PraxsmthConstant::Boolean(x), PraxsmthConstant::Boolean(y)) => {
                        Ok(PraxsmthConstant::Boolean(x && y))
                    }
                    (x, y) => bail!(
                        "And conditions must evaluate to boolean, got {:?} and {:?}",
                        x,
                        y
                    ),
                }
            }

            Expression::Or(x, y) => {
                let x = self.evaluate_expression(world, *x, bindings)?;
                let y = self.evaluate_expression(world, *y, bindings)?;
                match (x, y) {
                    (PraxsmthConstant::Boolean(x), PraxsmthConstant::Boolean(y)) => {
                        Ok(PraxsmthConstant::Boolean(x || y))
                    }
                    (x, y) => bail!(
                        "Or conditions must evaluate to boolean, got {:?} and {:?}",
                        x,
                        y
                    ),
                }
            }

            Expression::Is(x, y) => {
                let x = self.evaluate_expression(world, *x, bindings)?;
                let y = self.evaluate_expression(world, *y, bindings)?;
                Ok(PraxsmthConstant::Boolean(x == y))
            }

            Expression::Not(x) => {
                let res = self.evaluate_expression(world, *x, bindings)?;
                match res {
                    PraxsmthConstant::Boolean(b) => Ok(PraxsmthConstant::Boolean(!b)),
                    other => bail!("Not condition must evaluate to boolean, got {:?}", other),
                }
            }
        }
    }

    fn evaluate_expression_as_boolean(
        &self,
        world: &World,
        expression: Expression,
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

    pub fn check_condition(
        &self,
        world: &World,
        condition: &Condition,
        bindings: &Bindings,
    ) -> Result<bool> {
        let possible_bindings = self.solve_for_free_vars(world, &condition.expression, bindings)?;
        if possible_bindings.is_empty() {
            // No valid bindings, so condition is false
            return Ok(false);
        }

        match condition.resolution_method {
            ResolutionMethod::Any => {
                for binding in possible_bindings {
                    if self.evaluate_expression_as_boolean(
                        world,
                        condition.expression.clone(),
                        &binding,
                    )? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            ResolutionMethod::All => {
                for binding in possible_bindings {
                    if !self.evaluate_expression_as_boolean(
                        world,
                        condition.expression.clone(),
                        &binding,
                    )? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
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
        self.dialog_history.push(dialog.clone());
        Ok(dialog)
    }

    fn process_delete(
        &mut self,
        world: &mut World,
        sentence: &Sentence,
        bindings: &Bindings,
    ) -> Result<()> {
        let (query, args) = self
            .build_query(world, sentence, bindings)
            .with_context(|| format!("processing delete outcome {:?}", sentence))?;
        if !args.is_empty() {
            bail!("extra parameters in delete outcome {:?}", sentence);
        }

        let (edge, _) = self
            .lookup_relation(world, query)
            .with_context(|| format!("relation not found in delete outcome {:?}", sentence))?;
        world
            .remove_relation(edge.relation_handle.clone())
            .with_context(|| format!("removing relation in delete outcome {:?}", sentence))
    }

    fn process_update(
        &mut self,
        world: &mut World,
        sentence: &Sentence,
        value: &PraxsmthValue,
        bindings: &Bindings,
    ) -> Result<()> {
        let (query, args) = self
            .build_query(world, sentence, bindings)
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

        let (edge, _) = self
            .lookup_relation(world, query)
            .with_context(|| format!("relation not found in set outcome {:?}", sentence))?;

        let constant_value = match value {
            PraxsmthValue::Number(n) => PraxsmthConstant::Number(*n),
            PraxsmthValue::Boolean(b) => PraxsmthConstant::Boolean(*b),
            PraxsmthValue::Variant(v) => PraxsmthConstant::Variant(v.clone()),
            PraxsmthValue::String(s) => PraxsmthConstant::String(s.clone()),
            PraxsmthValue::Variable(sentence) => self
                .resolve_variable(world, sentence, bindings)
                .context("resolving variable for set outcome")?,
        };

        world
            .update_relation(
                edge.relation_handle.clone(),
                HashMap::from([(arg_name.clone(), constant_value)]),
            )
            .with_context(|| format!("applying set outcome {:?}", sentence))
    }

    fn process_increase(
        &mut self,
        _world: &mut World,
        _sentence: &Sentence,
        _amount: i64,
        _bindings: &Bindings,
    ) -> Result<()> {
        unimplemented!()
    }

    fn process_cycle(
        &mut self,
        _world: &mut World,
        _sentence: &Sentence,
        _amount: i64,
        _bindings: &Bindings,
    ) -> Result<()> {
        unimplemented!()
    }

    pub fn process_outcome(
        &mut self,
        world: &mut World,
        agent_name: &str,
        outcome: &PracticeOutcome,
        bindings: &Bindings,
    ) -> Result<Option<Dialog>> {
        match outcome {
            PracticeOutcome::Broadcast(string) => {
                return Ok(Some(self.process_print(world, None, string, bindings)?));
            }
            PracticeOutcome::Say(string) => {
                return Ok(Some(self.process_print(
                    world,
                    Some(agent_name),
                    string,
                    bindings,
                )?));
            }
            PracticeOutcome::Activate(agent_id) => {
                world.set_agent_active(&bindings.get_or_same(agent_id), true)
            }
            PracticeOutcome::Deactivate(agent_id) => {
                world.set_agent_active(&bindings.get_or_same(agent_id), false)
            }
            PracticeOutcome::Delete(sentence) => self.process_delete(world, sentence, bindings),
            PracticeOutcome::Set(declaration) => self
                .process_declaration(world, declaration, bindings)
                .map(|_| ()),
            PracticeOutcome::Update(sentence, value) => {
                self.process_update(world, sentence, value, bindings)
            }
            PracticeOutcome::Increase(sentence, amount) => {
                self.process_increase(world, sentence, *amount, bindings)
            }
            PracticeOutcome::Cycle(sentence, amount) => {
                self.process_cycle(world, sentence, *amount, bindings)
            }
        }?;
        Ok(None)
    }

    pub fn get_available_actions(
        &self,
        world: &World,
        agent_name: &str,
    ) -> Result<Vec<AvailableAction>> {
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

                        available_actions.push(AvailableAction {
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
        world: &mut World,
        available_action: &AvailableAction,
    ) -> Result<Vec<Dialog>> {
        let relation = world
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
        let outcomes = action.outcomes.clone();
        let action_name = action.name.clone();
        let bindings = bindings.clone();

        let mut dialog: Vec<Dialog> = vec![];

        for outcome in &outcomes {
            if let Some(new_dialog) = self
                .process_outcome(world, &actor_name, outcome, &bindings)
                .with_context(|| format!("processing outcome of action {}", action_name))?
            {
                dialog.push(new_dialog);
            }
        }

        Ok(dialog)
    }
}
