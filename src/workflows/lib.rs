use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
};

use serde::{Deserialize, Serialize};

// enum RecipeOutput {
//     Integer(i64),
//     Multiple(Vec<i64>),
//     Lookup(HashMap<String, i64>),
// }

type NodeID = i32;

#[derive(Serialize, Deserialize, Debug)]
// #[serde(deny_unknown_fields)]
pub struct Node {
    queue: String,
    service: String,
}

impl Node {
    pub fn all_outgoing(&self) -> HashSet<NodeID> {
        HashSet::new()
    }
}

#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Recipe {
    /// Recipe steps
    #[serde(flatten)]
    #[serde_as(as = "HashMap<serde_with::DisplayFromStr, _>")]
    nodes: HashMap<NodeID, Node>,
    /// The list of all nodes to start the graph processing, with optional initial parameters.
    start: Vec<(NodeID, Option<serde_json::Value>)>,
    /// Nodes to trigger when an error happens
    #[serde(default)]
    error: Vec<NodeID>,
}

#[derive(PartialEq, Hash, Eq, Copy, Clone)]
enum NodeVertex {
    Start,
    Error,
    Node(NodeID),
}

impl From<NodeID> for NodeVertex {
    fn from(value: NodeID) -> Self {
        NodeVertex::Node(value)
    }
}

impl Display for NodeVertex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeVertex::Start => write!(f, "start"),
            NodeVertex::Error => write!(f, "end"),
            NodeVertex::Node(x) => write!(f, "{x}"),
        }
    }
}

/// Generate a list of reachable nodes from a particular recipe vertex
///
/// # Errors
/// - A references node is missing
/// - The node graph is not a DAG, because it has cycles
fn all_reachable_dag_nodes(recipe: &Recipe, start: NodeVertex) -> Result<HashSet<NodeID>, String> {
    let mut visited_nodes = HashSet::new();
    let mut current_path = vec![start];

    'outer: while !current_path.is_empty() {
        let current_node = *current_path.last().unwrap();
        visited_nodes.insert(current_node.to_owned());

        // Get a list of all nodes going out from this one
        let outgoing_nodes: HashSet<i32> = match current_node {
            NodeVertex::Start => recipe.start.iter().map(|x| x.0).collect(),
            NodeVertex::Error => recipe.error.iter().copied().collect(),
            NodeVertex::Node(id) => recipe
                .nodes
                .get(&id)
                .ok_or(format!("Referenced node {id} is missing from recipe"))?
                .all_outgoing(),
        };
        // If any of these outgoing nodes are in our current path, then we've detected a cycle
        let current_path_set: HashSet<NodeVertex> = current_path.iter().cloned().collect();
        let cycle_nodes = outgoing_nodes
            .iter()
            .map(|x| NodeVertex::Node(*x))
            .collect::<HashSet<NodeVertex>>()
            .intersection(&current_path_set)
            .cloned()
            .collect::<Vec<NodeVertex>>();
        if !cycle_nodes.is_empty() {
            Err(format!(
                "Cycle in DAG detected from node {current_node} → {}",
                cycle_nodes.first().unwrap()
            ))?;
        }
        // Loop through all outgoing nodes that are not in the "all_nodex"
        for node in outgoing_nodes {
            // If we have not visited this node, then push and search it
            if !visited_nodes.contains(&NodeVertex::Node(node)) {
                current_path.push(NodeVertex::Node(node));
                continue 'outer;
            }
        }
        // If here, then none of the outgoing nodes are unvisited. pop it.
        current_path.pop();
    }

    // Extract the numeric nodes only to return
    Ok(visited_nodes
        .iter()
        .filter_map(|&x| match x {
            NodeVertex::Node(id) => Some(id),
            _ => None,
        })
        .collect())
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
    #[test]
    fn test_reachable_dag_nodes() {
        let recipe = Recipe {
            nodes: HashMap::from([
                (
                    1,
                    Node {
                        queue: String::from("some_queue"),
                        service: String::from("Some Service"),
                    },
                ),
                (
                    2,
                    Node {
                        queue: String::new(),
                        service: String::new(),
                    },
                ),
            ]),
            start: vec![(1, None)],
            error: vec![1],
        };

        assert!(all_reachable_dag_nodes(&recipe, NodeVertex::Start).is_ok());
        assert!(all_reachable_dag_nodes(&recipe, NodeVertex::Error).is_err());
    }
}
