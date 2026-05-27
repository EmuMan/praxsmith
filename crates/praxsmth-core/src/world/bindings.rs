use std::collections::HashMap;

use crate::values::Sentence;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Bindings {
    pub variables: HashMap<String, String>,
    pub self_id: Option<Sentence>,
}

impl Bindings {
    /// Constructs a new `Bindings` instance with the given variable mappings
    /// and optional self identifier.
    pub fn new(variables: HashMap<String, String>, self_id: Option<Sentence>) -> Self {
        Bindings { variables, self_id }
    }

    pub fn self_only(self_id: Sentence) -> Self {
        Bindings {
            variables: HashMap::new(),
            self_id: Some(self_id),
        }
    }

    /// Retrieves the value of a variable from the bindings, if it exists.
    pub fn get(&self, var: &str) -> Option<&String> {
        self.variables.get(var)
    }

    /// Retrieves the value of a variable from the bindings, or returns the
    /// variable name itself if it is not found in the bindings.
    ///
    /// TODO: COW?
    pub fn get_or_same(&self, var: &str) -> String {
        self.get(var).cloned().unwrap_or_else(|| var.to_string())
    }

    /// Inserts a new variable mapping into the bindings, replacing any
    /// existing mapping for the same variable name.
    pub fn insert(&mut self, var: String, value: String) {
        self.variables.insert(var, value);
    }

    /// Creates a new `Bindings` instance by extending the current bindings
    /// with additional variable mappings.
    pub fn with(&self, additions: HashMap<String, String>) -> Self {
        let mut new_variables = self.variables.clone();
        for (var, value) in additions {
            new_variables.insert(var, value);
        }
        Bindings {
            variables: new_variables,
            self_id: self.self_id.clone(),
        }
    }

    /// Attempts to merge the current bindings with another set of bindings. If
    /// there are any conflicting variable mappings (i.e. the same variable
    /// maps to different values in the two sets of bindings), then this method
    /// returns `None` to indicate that the merge failed. Otherwise, it returns
    /// a new `Bindings` instance containing the merged variable mappings and a
    /// self identifier if either of the original bindings had one,
    /// prioritizing self for that value.
    pub fn try_merge(&self, other: &Bindings) -> Option<Self> {
        let mut merged_variables = self.variables.clone();
        for (var, value) in &other.variables {
            if let Some(existing_value) = merged_variables.get(var) {
                if existing_value != value {
                    return None; // Conflict detected
                }
            } else {
                merged_variables.insert(var.clone(), value.clone());
            }
        }
        // If we get here, there are no conflicts
        Some(Bindings {
            variables: merged_variables,
            self_id: self.self_id.clone().or_else(|| other.self_id.clone()),
        })
    }
}

impl Default for Bindings {
    fn default() -> Self {
        Bindings {
            variables: HashMap::new(),
            self_id: None,
        }
    }
}

impl<'a> IntoIterator for &'a Bindings {
    type Item = (&'a String, &'a String);
    type IntoIter = std::collections::hash_map::Iter<'a, String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.variables.iter()
    }
}
