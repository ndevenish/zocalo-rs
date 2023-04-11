use std::collections::HashMap;

use serde::{Deserialize, Serialize};

// enum RecipeOutput {
//     Integer(i64),
//     Multiple(Vec<i64>),
//     Lookup(HashMap<String, i64>),
// }

#[derive(Serialize, Deserialize, Debug)]
// #[serde(deny_unknown_fields)]
pub struct Node {
    queue: String,
    service: String,
}

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Recipe {
    /// Recipe steps
    #[serde(flatten)]
    #[serde_as(as = "HashMap<serde_with::DisplayFromStr, _>")]
    nodes: HashMap<i32, Node>,
    /// The list of all nodes to start the graph processing, with optional initial parameters.
    start: Vec<(i32, Option<serde_json::Value>)>,
    /// Nodes to trigger when an error happens
    #[serde(default)]
    error: Vec<i32>,
}

impl Recipe {
    pub fn new() -> Recipe {
        Recipe {
            nodes: HashMap::new(),
            start: Vec::new(),
            error: Vec::new(),
        }
    }
    pub fn merge(_other: &Recipe) -> Recipe {
        unimplemented!();
    }
    pub fn validate(&self) -> Result<(), String> {
        // 1. Start node: Implicit, impossible to break.

        // 2. Empty start node. Could happen while mutating or creating.
        if self.start.is_empty() {
            return Err(String::from("No start node specified"));
        }

        // 2. All start nodes are tuples with length 2. Impossible to break.
        // 3. Start node points to itself - Impossible to break.
        // 4. Error nodes only point to numeric nodes - Impossible to break.
        // 5. All other nodes are numeric - Impossible to break.

        // 6. Detect cycles in "start" node
        // 7. Detect cycles in "end" node

        // 8. Make sure there are no unreferenced nodes

        Ok(())
    }
}

impl Default for Recipe {
    fn default() -> Self {
        Recipe::new()
    }
}

pub struct RecipeWrapper {}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::from_str;

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

    const RECIPE_B_JSON: &str = r#"{
        "1": {
            "service": "A service",
            "queue": "some.queue.{first}",
            "output": 2
        },
        "2": {
            "service": "C service",
            "queue": "third.queue"
        },
        "start": [[1, {}]]
    }"#;

    #[test]
    fn empty_recipe() {
        let _recipe = Recipe::new();
    }

    #[test]
    fn parse_recipe_a() {
        let recipe_a: Recipe = serde_json::from_str(RECIPE_A_JSON).unwrap();
        assert!(recipe_a.validate().is_ok());
        println!("{:?}", recipe_a);
    }
    #[test]
    fn parse_recipe_b() {
        let recipe_b: Recipe = serde_json::from_str(RECIPE_B_JSON).unwrap();
        assert!(recipe_b.validate().is_ok());
        println!("{:?}", recipe_b);
    }
    #[test]
    fn test_validation_errors() {
        assert!(from_str::<Recipe>(r#""#).is_err());
        // from_str::<Recipe>(r#"{}"#).unwrap();
    }
}
