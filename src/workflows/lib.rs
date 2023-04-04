use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// enum RecipeOutput {
//     Integer(i64),
//     Multiple(Vec<i64>),
//     Lookup(HashMap<String, i64>),
// }

#[derive(Serialize, Deserialize, Debug)]
pub struct Node {
    queue: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Recipe {
    /// Recipe steps
    nodes: HashMap<i32, Node>,
    /// The list of all nodes to start the graph processing, with optional initial parameters.
    start: HashMap<i32, Option<serde_json::Value>>,
    /// Nodes to trigger when an error happens
    error: Vec<i32>,
}

impl Recipe {
    pub fn new() -> Recipe {
        Recipe {
            nodes: HashMap::new(),
            start: HashMap::new(),
            error: Vec::new(),
        }
    }
    pub fn merge(_other: &Recipe) -> Recipe {
        unimplemented!();
    }
}

impl Default for Recipe {
    fn default() -> Self {
        Recipe::new()
    }
}

pub struct RecipeWrapper {}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;
    const RECIPE_A_JSON: &str = r#"{
        "1": {
            "service": "A service",
            "queue": "some.queue.{first}",
            "output": ["2"],
            "error": 2
        },
        "2": {
            "service": "B service",
            "queue": "another.queue.{name}"
        },
        "start": [ [1,{}] ],
        "error": [2]
    }"#;

    #[test]
    fn empty_recipe() {
        let _recipe = Recipe::new();
    }

    #[test]
    fn parse_recipe_a() {
        let _recipe: Recipe = serde_json::from_str(RECIPE_A_JSON).unwrap();
    }
}
