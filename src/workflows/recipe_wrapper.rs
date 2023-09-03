use serde::{Deserialize, Serialize};

use crate::{NodeID, Recipe};

/// An in-progress workflow
///
/// Contains the recipe for the workflow, along with:
/// - A marker of where the workflow currently is
/// - The path it has taken to get here
/// - The payload associated with the message e.g. what the last service
///   sent (presumably to the currently-running service)
#[derive(Serialize, Deserialize, Debug)]
pub struct RecipeWrapper {
    recipe: Recipe,
    #[serde(alias = "recipe-pointer")]
    recipe_pointer: NodeID,
    #[serde(alias = "recipe-path")]
    recipe_path: Vec<NodeID>,
    environment: serde_json::Map<String, serde_json::Value>,
    payload: serde_json::Value,
}
