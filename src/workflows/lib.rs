use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    str::FromStr,
};

use serde::{de::Visitor, Deserialize, Serialize};
use thiserror::Error;

#[derive(Serialize, Deserialize, Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub struct NodeID(i64);

impl From<i64> for NodeID {
    fn from(value: i64) -> Self {
        NodeID(value)
    }
}
impl FromStr for NodeID {
    type Err = <i64 as FromStr>::Err;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(NodeID(s.parse()?))
    }
}
impl Display for NodeID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// Enum for intermediate parsing of NodeID. In the recipe, this can be
/// represented as both a numeric string, and an integer. Using this
/// intermediate format lets us just rely on serde default parsing.
#[derive(Deserialize)]
#[serde(untagged)]
enum IntermediateNodeID {
    Int(NodeID),
    String(String),
}

/// Represents a set of outputs for a node in a recipe.
///
/// There are three kinds:
/// - `None`: This recipe node doesn't lead anywhere else
/// - `Direct`: This recipe node has a direct list of onward nodes
/// - `Lookup`: This recipe has multiple possible onward nodes, depending
///             upon the behaviour of the actual service.
#[derive(Debug)]
pub enum NodeOutput {
    None,
    Direct(Vec<NodeID>),
    Lookup(HashMap<String, Vec<NodeID>>),
}

impl NodeOutput {
    fn new() -> Self {
        NodeOutput::None
    }
    /// A set of all possible nodes from this output set
    fn all_nodes(&self) -> HashSet<NodeID> {
        match self {
            NodeOutput::Direct(v) => v.iter().cloned().collect(),
            NodeOutput::Lookup(m) => m.values().flatten().cloned().collect(),
            NodeOutput::None => HashSet::new(),
        }
    }
    fn is_empty(&self) -> bool {
        match self {
            NodeOutput::None => true,
            NodeOutput::Direct(v) => v.is_empty(),
            NodeOutput::Lookup(m) => m.is_empty(),
        }
    }
}
impl Default for NodeOutput {
    fn default() -> Self {
        NodeOutput::new()
    }
}

struct NodeOutputVisitor;

impl<'de> Visitor<'de> for NodeOutputVisitor {
    type Value = NodeOutput;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter
            .write_str("Node ID (int or string), sequence of Node ID, or map from name to node ID")
    }

    fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(NodeOutput::Direct(vec![NodeID(v)]))
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(NodeOutput::Direct(vec![NodeID(
            v.try_into().map_err(|e| E::custom(e))?,
        )]))
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(NodeOutput::Direct(vec![v
            .parse()
            .map_err(|e| E::custom(e))?]))
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::SeqAccess<'de>,
    {
        let mut output = Vec::with_capacity(seq.size_hint().unwrap_or(0));

        while let Some(elem) = seq.next_element()? {
            output.push(match elem {
                IntermediateNodeID::Int(e) => e,
                IntermediateNodeID::String(s) => s.parse().map_err(serde::de::Error::custom)?,
            })
        }
        // If we are an empty direct node, just class this as "None"
        if output.is_empty() {
            Ok(NodeOutput::None)
        } else {
            Ok(NodeOutput::Direct(output))
        }
    }
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut output = HashMap::with_capacity(map.size_hint().unwrap_or(0));
        while let Some((key, value)) = map.next_entry()? {
            output.insert(
                key,
                match value {
                    NodeOutput::Direct(x) => x,
                    NodeOutput::Lookup(_) => {
                        return Err(serde::de::Error::custom(
                            "Do not support recursive output maps",
                        ))
                    }
                    NodeOutput::None => {
                        return Err(serde::de::Error::custom(
                            "Do not understand receiving None output in an output map",
                        ))
                    }
                },
            );
        }
        // If we had no entries, return None instead of an empty Lookup map.
        if output.is_empty() {
            Ok(NodeOutput::None)
        } else {
            Ok(NodeOutput::Lookup(output))
        }
    }
}

impl<'de> Deserialize<'de> for NodeOutput {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_any(NodeOutputVisitor {})
    }
}

impl Serialize for NodeOutput {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            NodeOutput::None => panic!("Should never try to serialize an empty NodeOutput"),
            NodeOutput::Direct(v) => {
                if v.len() == 1 {
                    serializer.serialize_i64(v.first().unwrap().0)
                } else {
                    serializer.collect_seq(v.iter())
                }
            }
            NodeOutput::Lookup(m) => serializer.collect_map(m.iter()),
        }
    }
}

/// A Recipe node. Describes the destination queue, connections and
/// other data that the service instance will use to process the node.
#[derive(Serialize, Deserialize, Debug)]
// #[serde(deny_unknown_fields)]
pub struct Node {
    /// The message broken queue that this recipe will be posted to
    pub queue: String,
    /// The "Name" of the service that is responsible for this node
    pub service: Option<String>,
    /// Onward nodes, that messages from this node can be sent
    #[serde(default)]
    #[serde(skip_serializing_if = "NodeOutput::is_empty")]
    pub output: NodeOutput,
    /// Nodes that will be triggered if a (node-service-defined) error occurs
    #[serde(default)]
    #[serde(skip_serializing_if = "NodeOutput::is_empty")]
    pub error: NodeOutput,
}

impl Default for Node {
    fn default() -> Self {
        Node::new()
    }
}
impl Node {
    pub fn new() -> Self {
        Node {
            queue: String::new(),
            service: None,
            output: NodeOutput::None,
            error: NodeOutput::None, // error: None,
        }
    }
    /// Generate a list of every possible declared destination node from
    /// this one. This covers both "output" and "error" fields.
    pub fn all_outgoing(&self) -> HashSet<NodeID> {
        let mut output = HashSet::new();
        output.extend(self.output.all_nodes());
        output.extend(self.error.all_nodes());
        output
    }
}

/// A self-contained description of a workflow.
///
///
#[serde_with::serde_as]
#[derive(Serialize, Deserialize, Debug)]
pub struct Recipe {
    /// Node steps of the recipe. Each node describes a single, modelled "step"
    /// of the recipe, that is sent the recipe state, and can read the recipe
    /// in order to decide where to dispatch the next step of the workflow.
    #[serde(flatten)]
    #[serde_as(as = "HashMap<serde_with::DisplayFromStr, _>")]
    pub nodes: HashMap<NodeID, Node>,
    /// Which nodes to start the workflow from, with the initial
    /// associated payload state that will sent with the recipe wrapper
    pub start: Vec<(NodeID, Option<serde_json::Value>)>,
    /// If an error occurs during processing, which nodes should be triggered?
    #[serde(default)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub error: Vec<NodeID>,
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

#[derive(Error, Debug)]
pub enum RecipeError {
    #[error("A referenced node ({0}) is missing from the recipe")]
    MissingNode(NodeID),
    #[error("A node ({0}) exists but is not referenced")]
    UnreferencedNode(NodeID),
    #[error("The recipe has node reference-cycles, caused by link from {0} ↦ {1}")]
    RecipeHasCycles(NodeID, NodeID),
    #[error("No start node has been specified")]
    NoStartNode,
}

/// Generate a list of reachable nodes from a particular recipe vertex
///
/// # Errors
/// - A referenced node is missing
/// - The node graph is not a DAG, because it has cycles
fn all_reachable_dag_nodes(
    recipe: &Recipe,
    start: NodeVertex,
) -> Result<HashSet<NodeID>, RecipeError> {
    let mut visited_nodes = HashSet::new();
    let mut current_path = vec![start];

    'outer: while !current_path.is_empty() {
        let current_node = *current_path.last().unwrap();
        visited_nodes.insert(current_node.to_owned());

        // Get a list of all nodes going out from this one
        let outgoing_nodes: HashSet<NodeID> = match current_node {
            NodeVertex::Start => recipe.start.iter().map(|x| x.0).collect(),
            NodeVertex::Error => recipe.error.iter().copied().collect(),
            NodeVertex::Node(id) => recipe
                .nodes
                .get(&id)
                .ok_or(RecipeError::MissingNode(id))?
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
            let current_node_id = match current_node {
                NodeVertex::Node(x) => Ok(x),
                _ => Err(()),
            }
            .unwrap();
            let target_node_id = match cycle_nodes.first().unwrap() {
                NodeVertex::Node(x) => Ok(*x),
                _ => Err(()),
            }
            .unwrap();

            return Err(RecipeError::RecipeHasCycles(
                current_node_id,
                target_node_id,
            ));
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
    pub fn validate(&self) -> Result<(), RecipeError> {
        // Numbered checks from python implementation;

        // 1. Start node exists: Implicit, impossible to break.

        // 2. Empty start node. Could happen while mutating or creating.
        if self.start.is_empty() {
            return Err(RecipeError::NoStartNode);
        }

        // 2. All start nodes are tuples with length 2. Impossible to break.
        // 3. Start node points to itself - Impossible to break.
        // 4. Error nodes only point to numeric nodes - Impossible to break.
        // 5. All other nodes are numeric - Impossible to break.

        // 6. Detect cycles in "start" node
        let start_accessible = all_reachable_dag_nodes(self, NodeVertex::Start)?;
        // 7. Detect cycles in "end" node
        let error_accessible = all_reachable_dag_nodes(self, NodeVertex::Error)?;

        // 8. Make sure there are no unreferenced nodes
        let keys: HashSet<NodeID> = self.nodes.keys().cloned().collect();
        let all_referenced_nodes: HashSet<NodeID> = start_accessible
            .iter()
            .chain(&error_accessible)
            .cloned()
            .collect();
        // The case where a node is referenced in the DAG but not present is
        // handled by the all_reachable_dag_nodes check. Thus, we only need
        // to check here for unreferenced ndoes
        if keys != all_referenced_nodes && keys.is_superset(&all_referenced_nodes) {
            let mut unreferenced = keys.difference(&all_referenced_nodes);
            return Err(RecipeError::UnreferencedNode(*unreferenced.next().unwrap()));
        }
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
        let recipe_a: Recipe = from_str(RECIPE_A_JSON).unwrap();
        assert!(recipe_a.validate().is_ok());
        println!("{:?}", recipe_a);
    }
    #[test]
    fn parse_recipe_b() {
        let recipe_b: Recipe = from_str(RECIPE_B_JSON).unwrap();
        assert!(recipe_b.validate().is_ok());
        println!("{:?}", recipe_b);
    }
    #[test]
    fn test_validation_errors() {
        // Possible validation failures:
        // 2. Empty start node. Could happen while mutating or creating.
        assert!(matches!(
            from_str::<Recipe>(r#"{"start":[]}"#)
                .unwrap()
                .validate()
                .unwrap_err(),
            RecipeError::NoStartNode
        ));
        // 6. Detect cycles in "start" node
        assert!(matches!(
            from_str::<Recipe>(r#"{"start": [[1, []]], "1": {"output": 2, "queue": "q1"}, "2": {"output": 1, "queue": "q2"}}"#).unwrap().validate().unwrap_err(),
            RecipeError::RecipeHasCycles(_, _)
        ));
        // 7. Detect cycles in "error" node
        assert!(matches!(
            from_str::<Recipe>(
                r#"{
                "start": [[3, []]],
                "error": [1],
                "1": {"output": 2, "queue": "q1"},
                "2": {"output": 1, "queue": "q2"},
                "3": {"queue": "q3"}
            }"#
            )
            .unwrap()
            .validate()
            .unwrap_err(),
            RecipeError::RecipeHasCycles(_, _)
        ));
        // 8. Make sure there are no unreferenced nodes
        assert!(matches!(
            from_str::<Recipe>(
                r#"{
                "start": [[1, []]],
                "1": {"queue": "q1"},
                "2": {"queue": "q2"}
            }"#
            )
            .unwrap()
            .validate()
            .unwrap_err(),
            RecipeError::UnreferencedNode(2)
        ));
        // And that there are no missing referenced nodes
        assert!(matches!(
            from_str::<Recipe>(
                r#"{
            "start": [[1, []]],
            "1": {"queue": "q1", "output": 2}
        }"#
            )
            .unwrap()
            .validate()
            .unwrap_err(),
            RecipeError::MissingNode(2)
        ));
    }

    #[test]
    fn test_impossible_deserialization_fails() {
        // Test cases that are explicitly validated by the python implementation,
        // but should be impossible to deserialize here

        // Node without a queue
        assert!(from_str::<Recipe>(r#"{"start": [[1, []]], "1": {}}"#).is_err());

        // Missing Start node
        assert!(from_str::<Recipe>(r#"{"1": {"queue": "some"}}"#).is_err());

        // All start nodes are tuples with length 2
        assert!(from_str::<Recipe>(r#"{"start": 1, "1": {"queue": "some"}}"#).is_err());
        assert!(from_str::<Recipe>(r#"{"start": [1], "1": {"queue": "some"}}"#).is_err());
        assert!(from_str::<Recipe>(r#"{"start": [[1]], "1": {"queue": "some"}}"#).is_err());
        assert!(from_str::<Recipe>(r#"{"start": [[1, [], 1]], "1": {"queue": "some"}}"#).is_err());

        // Start node points to itself
        assert!(from_str::<Recipe>(r#"{"start": [["start", []]}"#).is_err());

        // Error nodes only point to numeric nodes
        assert!(from_str::<Recipe>(
            r#"{"start": [[1, []], "end": ["start"], "1": {"queue": "q"}}"#
        )
        .is_err());
        // Other nodes are non-numeric
        assert!(from_str::<Recipe>(
            r#"{"start": [[1, []], "1": {"queue": "some"}, "another": {"queue": "q2"}}"#
        )
        .is_err());
    }

    #[test]
    fn test_parsing_mapped_outputs() {
        let recipe: Recipe = from_str(
            r#"{
            "1": {
                "service": "Test outputs",
                "queue": "some",
                "output": {
                    "one": 2,
                    "all": 3
                }
            },
            "2": {"service": "service 2", "queue": "service_2"},
            "3": {"service": "service 3", "queue": "service_3"},
            "start": [[1, {}]]
        }"#,
        )
        .unwrap();
        println!("{recipe:#?}");

        assert!(matches!(
            &recipe.nodes.get(&1).unwrap().output,
            NodeOutput::Lookup(_x)
        ));

        let output = match &recipe.nodes.get(&1).unwrap().output {
            NodeOutput::Lookup(m) => m,
            _ => panic!("Unknown node type"),
        };
        let expected = HashMap::from([
            ("one".to_owned(), vec![2 as NodeID]),
            ("all".to_owned(), vec![3 as NodeID]),
        ]);

        // Does this just work?
        assert!(&expected == output);
    }

    #[test]
    fn test_reachable_dag_nodes() {
        let mut recipe = from_str(
            r#"{
            "1": {
                "queue": "some"
            },
            "2": {
                "queue": "some_2",
                "output": 3
            },
            "3": {
                "queue": "some_3",
                "output": 2
            },
            "start": [[1, {}]],
            "error": [2]
        }"#,
        )
        .unwrap();

        assert!(all_reachable_dag_nodes(&recipe, NodeVertex::Start).is_ok());
        // This has cycles, so should fail
        assert!(all_reachable_dag_nodes(&recipe, NodeVertex::Error).is_err());

        // Check reassigning this to an unknown node should still error
        recipe.nodes.get_mut(&(3 as NodeID)).unwrap().output = NodeOutput::Direct(vec![4]);
        assert!(all_reachable_dag_nodes(&recipe, NodeVertex::Error).is_err());

        // Check that we pick up a node pointing to itself
        recipe.nodes.get_mut(&(3 as NodeID)).unwrap().output = NodeOutput::Direct(vec![3]);
        assert!(all_reachable_dag_nodes(&recipe, NodeVertex::Error).is_err());
    }

    #[test]
    fn test_serialize() {
        let recipe_a: Recipe = from_str(RECIPE_A_JSON).unwrap();
        let reserial = serde_json::to_string(&recipe_a).unwrap();
        println!("Recipe In: {RECIPE_A_JSON}\nParsed: {recipe_a:#?}\nSerialized: {reserial}");

        let recipe_b: Recipe = from_str(RECIPE_B_JSON).unwrap();
        let reserial = serde_json::to_string(&recipe_b).unwrap();
        println!("Recipe In: {RECIPE_B_JSON}\nParsed: {recipe_b:#?}\nSerialized: {reserial}");
    }

    #[test]
    fn test_complex_ispyb_autoprocessing_xia2_dials_eiger_cluster() {
        let s = r#"{
            "1": [
              "Recipe for processing data collections with xia2 and the DIALS pipeline.",
              "This registers an AutoProcProgram with ISPyB and passes the AutoProcProgramId",
              "to downstream services via a RecipeWrapper environment variable.",
              "Then xia2 is run on the  on STFC/IRIS cloud with the dials pipeline taking",
              "processing parameters and sweep selection into account."
            ],
            "1": { "service": "DLS ISPyB connector",
                   "queue": "ispyb_connector",
                   "parameters": { "ispyb_command": "multipart_message",
                                   "dcid": "{ispyb_dcid}",
                                   "ispyb_command_list": [
                                     { "ispyb_command": "register_processing",
                                       "program": "xia2 dials",
                                       "cmdline": "XIA2 DIALS (ap-zoc)",
                                       "environment": "{$REPLACE:ispyb_reprocessing_parameters}",
                                       "rpid": "{ispyb_process}",
                                       "store_result": "ispyb_autoprocprogram_id"
                                     },
                                     { "ispyb_command": "upsert_integration",
                                       "program_id": "$ispyb_autoprocprogram_id",
                                       "store_result": "ispyb_integration_id"
                                     }
                                   ]
                                 },
                   "output": [2,13]
                 },
            "2": { "service": "DLS cluster submission",
                   "queue": "cluster.submission",
                   "parameters": { "cluster": "{ispyb_preferred_datacentre}",
                                   "cluster_project": "{ispyb_beamline}",
                                   "cluster_queue": "high",
                                   "cluster_submission_parameters": "-N zoc-xia2-dials-setup -l h_rt=1:00:00",
                                   "cluster_commands": [
          "module load dials",
          "echo dlstbx.wrap -t PikaTransport --wrap xia2_setup --recipewrapper \"$RECIPEWRAP\" >runinfo",
          "dlstbx.wrap -t PikaTransport --wrap xia2_setup --recipewrapper \"$RECIPEWRAP\""
                                   ],
                                   "recipewrapper": "{ispyb_working_directory}/.recipewrap",
                                   "workingdir": "{ispyb_working_directory}/.launch"
                                 },
                   "wrapper": { "task_information": "{ispyb_beamline}" },
                   "job_parameters": {
                     "ispyb_parameters": "{$REPLACE:ispyb_reprocessing_parameters}",
                     "timeout": null,
                     "working_directory": "{ispyb_working_directory}",
                     "create_symlink": "xia2-dials"
                   },
                   "output": {
                     "success": [3,14],
                     "failure": 6
                   }
                 },
            "3": { "service": "DLS cluster submission",
                   "queue": "cluster.submission",
                   "parameters": { "cluster": "{ispyb_preferred_datacentre}",
                                   "cluster_project": "{ispyb_beamline}",
                                   "cluster_queue": "default",
                                   "cluster_submission_parameters": "-N zoc-xia2-dials -pe smp 16-20 -l mfree=4G -l h_rt=6:00:00",
                                   "cluster_commands": [
          "module load dials",
          "echo dlstbx.wrap -t PikaTransport --wrap xia2_run --recipewrapper \"$RECIPEWRAP\" >runinfo",
          "dlstbx.wrap -t PikaTransport --wrap xia2_run --recipewrapper \"$RECIPEWRAP\""
                                   ],
                                   "recipewrapper": "{ispyb_working_directory}/.recipewrap",
                                   "workingdir": "{ispyb_working_directory}/.launch"
                                 },
                   "wrapper": { "task_information": "{ispyb_beamline}" },
                   "job_parameters": {
                     "xia2": { "images": "{ispyb_images}",
                               "min_images": 3,
                               "pipeline": "dials",
                               "nproc": 20,
                               "dynamic_shadowing": false,
                               "read_all_image_headers": false,
                               "anomalous": true,
                               "project": "{ispyb_project}",
                               "crystal": "{ispyb_crystal}",
                               "trust_beam_centre": true
                             },
                     "working_directory": "{ispyb_working_directory}",
                     "ispyb_parameters": "{$REPLACE:ispyb_reprocessing_parameters}",
                     "timeout": null,
                     "create_symlink": "xia2-dials"
                   },
              "output": {
                "success": 4,
                "failure": 4
              }
                },
            "4": { "service": "DLS cluster submission",
                   "queue": "cluster.submission",
                   "parameters": { "cluster": "{ispyb_preferred_datacentre}",
                                   "cluster_project": "{ispyb_beamline}",
                                   "cluster_queue": "high",
                                   "cluster_submission_parameters": "-N zoc-xia2-dials-results -l mfree=400M -l h_rt=1:00:00",
                                   "cluster_commands": [
          "module load dials",
          "echo dlstbx.wrap -t PikaTransport --wrap xia2_results --recipewrapper \"$RECIPEWRAP\" >runinfo",
          "dlstbx.wrap -t PikaTransport --wrap xia2_results --recipewrapper \"$RECIPEWRAP\""
                                   ],
                                   "recipewrapper": "{ispyb_working_directory}/.recipewrap",
                                   "workingdir": "{ispyb_working_directory}/.launch"
                                 },
                   "wrapper": { "task_information": "{ispyb_beamline}" },
                   "job_parameters": {
                     "ispyb_parameters": "{$REPLACE:ispyb_reprocessing_parameters}",
                     "timeout": null,
                     "working_directory": "{ispyb_working_directory}/xia2-dials",
                     "results_directory": "{ispyb_results_directory}/xia2-dials",
                     "create_symlink": "xia2-dials",
                     "store_xtriage_results": true,
                     "pipeline": "dials",
                     "dcid": "{ispyb_dcid}",
                     "dc_end_time": "{ispyb_dc_info[endTime]}"
                   },
                   "output": {
                     "success": 5,
                     "failure": 6,
                     "ispyb": 7,
                     "result-individual-file": 8
                   }
                 },
            "13": { "service": "DLS ISPyB connector",
                   "queue": "ispyb_connector",
                   "parameters": { "ispyb_command": "update_processing_status",
                                   "program_id": "$ispyb_autoprocprogram_id",
                                   "message": "starting" }
                 },
            "14": { "service": "DLS ISPyB connector",
                   "queue": "ispyb_connector",
                   "parameters": { "ispyb_command": "update_processing_status",
                                   "program_id": "$ispyb_autoprocprogram_id",
                                   "message": "processing" }
                 },
            "5": { "service": "DLS ISPyB connector",
                   "queue": "ispyb_connector",
                   "parameters": { "ispyb_command": "update_processing_status",
                                   "program_id": "$ispyb_autoprocprogram_id",
                                   "message": "processing successful",
                                   "status": "success" }
                 },
            "6": { "service": "DLS ISPyB connector",
                   "queue": "ispyb_connector",
                   "parameters": { "ispyb_command": "update_processing_status",
                                   "program_id": "$ispyb_autoprocprogram_id",
                                   "message": "processing failure",
                                   "status": "failure" }
                 },
            "7": { "service": "DLS ISPyB connector",
                   "queue": "ispyb_connector",
                   "parameters": { "ispyb_command": "multipart_message",
                                   "dcid": "{ispyb_dcid}",
                                   "integration_id": "$ispyb_integration_id",
                                   "program_id": "$ispyb_autoprocprogram_id"
                                 },
                   "output": [9,10,12,16]
                 },
            "8": { "service": "DLS ISPyB connector",
                   "queue": "ispyb_connector",
                   "parameters": { "ispyb_command": "add_program_attachment",
                                   "program_id": "$ispyb_autoprocprogram_id"
                                 }
                 },
            "9": { "service": "DLS Trigger",
                   "queue": "trigger",
                   "parameters": { "target": "dimple",
                                   "dcid": "{ispyb_dcid}",
                                   "comment": "DIMPLE triggered by automatic xia2-dials",
                                   "automatic": true,
                                   "scaling_id": "$ispyb_autoprocscaling_id",
                                   "pdb": "{$REPLACE:ispyb_pdb}",
                                   "user_pdb_directory": "{ispyb_visit_directory}/processing/pdb",
                                   "pdb_tmpdir": "{ispyb_visit_directory}/tmp/pdb",
                                   "mtz": "{ispyb_results_directory}/xia2-dials/DataFiles/{ispyb_project}_{ispyb_crystal}_free.mtz"
                                 }
                 },
            "10": { "service": "DLS ISPyB connector",
                   "queue": "ispyb_connector",
                   "parameters": { "ispyb_command": "retrieve_programs_for_job_id",
                                   "rpid": "{ispyb_process}",
                                   "store_result": "ispyb_programs_all"
                                 },
                   "output": 11
                 },
            "11": { "service": "DLS Trigger",
                   "queue": "trigger",
                   "parameters": { "target": "big_ep",
                                   "dcid": "{ispyb_dcid}",
                                   "comment": "big_ep triggered by automatic xia2-dials",
                                   "automatic": true,
                                   "program_id": "$ispyb_autoprocprogram_id",
                                   "spacegroup": "{ispyb_space_group}",
                                   "diffraction_plan_info": "{$REPLACE:ispyb_diffraction_plan}",
                                   "xia2 dials": { "data": "{ispyb_results_directory}/xia2-dials/DataFiles/{ispyb_project}_{ispyb_crystal}_free.mtz",
                                                   "scaled_unmerged_mtz": "{ispyb_results_directory}/xia2-dials/DataFiles/{ispyb_project}_{ispyb_crystal}_scaled_unmerged.mtz",
                                                   "path_ext": "xia2/dials-run"
                                                 }
                                 }
                 },
            "12": { "service": "DLS Trigger",
                   "queue": "trigger",
                   "parameters": { "target": "multiplex",
                                   "dcid": "{ispyb_dcid}",
                                   "wavelength": "{ispyb_dc_info[wavelength]}",
                                   "comment": "xia2.multiplex triggered by automatic xia2-dials",
                                   "automatic": true,
                                   "ispyb_parameters": "{$REPLACE:ispyb_reprocessing_parameters}",
                                   "related_dcids": "{$REPLACE:ispyb_related_dcids}",
                                   "diffraction_plan_info": "{$REPLACE:ispyb_diffraction_plan}",
                                   "backoff-delay": 8,
                                   "backoff-max-try": 10,
                                   "backoff-multiplier": 2
                                 }
                 },
            "16": { "service": "DLS Trigger",
                  "queue": "trigger",
                  "parameters": { "target": "mrbump",
                                  "dcid": "{ispyb_dcid}",
                                  "comment": "MrBUMP triggered by automatic xia2-dials",
                                  "automatic": true,
                                  "pdb": "{$REPLACE:ispyb_pdb}",
                                  "user_pdb_directory": "{ispyb_visit_directory}/processing/pdb",
                                  "pdb_tmpdir": "{ispyb_visit_directory}/tmp/pdb",
                                  "protein_info": "{$REPLACE:ispyb_protein_info}",
                                  "scaling_id": "$ispyb_autoprocscaling_id",
                                  "hklin":  "{ispyb_results_directory}/xia2-dials/DataFiles/{ispyb_project}_{ispyb_crystal}_free.mtz"
                                }
                },
            "start": [
                [1, []]
            ]
          }"#;
        let v: serde_json::Value = from_str(s).unwrap();
        // Let's try this from value
        let r: Recipe = serde_json::from_value(v).unwrap();
        // let r: Recipe = from_str(s).unwrap();
        println!("{r:#?}");
    }

    #[test]
    fn test_redefine_key() {
        let r = from_str::<Recipe>(
            r#"{
            "1": {"queue": "some_first"},
            "1": {"queue": "some_second"},
            "start": [[1, []]]
        }"#,
        )
        .unwrap();
        println!("{r:#?}");
    }
}
