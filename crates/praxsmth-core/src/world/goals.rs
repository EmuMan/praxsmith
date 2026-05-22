use crate::expressions::Expression;

#[derive(Debug, Clone, Default)]
pub enum GoalMeasurement {
    #[default]
    Exists,
    Delta,
}

#[derive(Debug, Clone)]
pub struct Goal {
    pub weight: f64,
    pub measurement: GoalMeasurement,
    pub expression: Expression,
}
