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
    /// The current in-progress recipe
    pub recipe: Recipe,
    #[serde(alias = "recipe-pointer")]
    /// The current recipe node
    pub recipe_pointer: NodeID,
    /// The route this recipe has previously taken
    #[serde(alias = "recipe-path")]
    pub recipe_path: Vec<NodeID>,
    /// Recipe-wide key-value configurations
    pub environment: serde_json::Map<String, serde_json::Value>,
    /// The message content for this node of the recipe
    pub payload: serde_json::Value,
}
