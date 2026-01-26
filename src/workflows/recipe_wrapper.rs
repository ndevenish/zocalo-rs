use serde::{Deserialize, Serialize};

use super::{NodeID, Recipe};

/// An in-progress workflow
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
    /// The message content for the targeted node of the recipe
    pub payload: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use super::RecipeWrapper;

    const RECIPE_WRAPPER: &str = r#"    {
        "recipe": {
            "1": {
                "service": "A service",
                "queue": "some.queue.{first}",
                "output": [
                    "2"
                ],
                "error": 2
            },
            "2": {
                "service": "B service",
                "queue": "another.queue.{name}",
                "output": {
                    "all": "3"
                }
            },
            "3": {
                "service": "C Service",
                "queue": "third"
            },
            "start": [
                [
                    1,
                    {}
                ]
            ],
            "error": [
                2
            ]
        },
        "environment": {
            "ID": "fff0b833-a379-4ca5-b102-61ae87958393",
            "ispyb_autoproc_id": 13785026,
            "ispyb_autoprocprogram_id": 97260023,
            "ispyb_autoprocscaling_id": 13784840,
            "ispyb_integration_id": 19771526
        },
        "payload": {
            "result": [
                {
                    "id": 97260023,
                    "jobId": 17598926,
                    "message": "processing successful",
                    "programs": "xia2.multiplex",
                    "status": 1
                }
            ]
        },
        "recipe-path": [],
        "recipe-pointer": 1
    }"#;

    #[test]
    fn read_test_wrapper() {
        let rw: RecipeWrapper = serde_json::from_str(RECIPE_WRAPPER).unwrap();
        println!("{rw:#?}");
    }
}
