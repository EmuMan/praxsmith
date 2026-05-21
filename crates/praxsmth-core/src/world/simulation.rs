use std::collections::HashMap;

use anyhow::{Context, Result, bail};

use crate::{
    anyhow_ext::ResultOptionExt,
    definitions::{
        PraxsmthConstant, PraxsmthValue, Sentence,
        types::{Effect, Expression, PraxsmthTypeData},
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
        world: &mut WorldTxn,
        declaration: &Declaration,
        bindings: &Bindings,
    ) -> Result<RelationHandle> {
        let (query, args) = self
            .build_query(world.inner(), &declaration.sentence, bindings)
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
                PraxsmthValue::Variable(sentence) => {
                    self.resolve_variable(world, &sentence, bindings)
                }
            },

            Expression::And(x, y) => {
                let x = self.evaluate_expression(world, x.as_ref(), bindings)?;
                let y = self.evaluate_expression(world, y.as_ref(), bindings)?;
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
                let x = self.evaluate_expression(world, x.as_ref(), bindings)?;
                let y = self.evaluate_expression(world, y.as_ref(), bindings)?;
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

            Expression::ForAll(..) => {
                unimplemented!()
            }

            Expression::Any(..) => {
                unimplemented!()
            }
        }
    }

    fn evaluate_expression_as_boolean(
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

    pub fn check_condition(
        &self,
        _world: &World,
        _condition: &Expression,
        _bindings: &Bindings,
    ) -> Result<bool> {
        unimplemented!()
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
        let (query, args) = self
            .build_query(world.inner(), sentence, bindings)
            .with_context(|| format!("processing delete outcome {:?}", sentence))?;
        if !args.is_empty() {
            bail!("extra parameters in delete outcome {:?}", sentence);
        }

        let (edge, _) = self
            .lookup_relation(world.inner(), query)
            .with_context(|| format!("relation not found in delete outcome {:?}", sentence))?;
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
        let (query, args) = self
            .build_query(world.inner(), sentence, bindings)
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
            .lookup_relation(world.inner(), query)
            .with_context(|| format!("relation not found in set outcome {:?}", sentence))?;

        let constant_value = match value {
            PraxsmthValue::Number(n) => PraxsmthConstant::Number(*n),
            PraxsmthValue::Boolean(b) => PraxsmthConstant::Boolean(*b),
            PraxsmthValue::Variant(v) => PraxsmthConstant::Variant(v.clone()),
            PraxsmthValue::String(s) => PraxsmthConstant::String(s.clone()),
            PraxsmthValue::Variable(sentence) => self
                .resolve_variable(world.inner(), sentence, bindings)
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
    fn evaluate_goal(&self, _world: &World, goal: &Goal, _bindings: &Bindings) -> Result<f64> {
        unimplemented!();

        let evaluations: Vec<PraxsmthConstant> = vec![];

        let mut total_weight = 0.0;
        match goal.measurement {
            GoalMeasurement::Exists => {
                for evaluation in evaluations {
                    match evaluation {
                        PraxsmthConstant::Boolean(b) => {
                            if b {
                                total_weight += goal.weight;
                            }
                        }
                        other => bail!(
                            "goal expression must evaluate to boolean for Exists measurement, got {:?}",
                            other
                        ),
                    }
                }
            }
            GoalMeasurement::Delta => {
                for evaluation in evaluations {
                    match evaluation {
                        PraxsmthConstant::Number(n) => {
                            total_weight += n * goal.weight;
                        }
                        other => bail!(
                            "goal expression must evaluate to number for Increase/Decrease measurement, got {:?}",
                            other
                        ),
                    }
                }
            }
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
