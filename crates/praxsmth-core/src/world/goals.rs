use std::fmt;

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

impl fmt::Display for Goal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let measurement_str = match self.measurement {
            GoalMeasurement::Exists => "",
            GoalMeasurement::Delta => "delta ",
        };
        write!(
            f,
            "goal ({}): {}{}",
            self.weight, measurement_str, self.expression
        )
    }
}
