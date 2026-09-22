use std::{collections::HashMap, fmt::{Display, Write}};
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::DynamicError;

#[derive(Debug, Deserialize, Serialize)]
pub struct Preset {
    #[serde(rename = "audioGraph")]
    pub audio_graph: AudioGraph,

    pub info: PresetInfo,

    #[serde(rename = "nodeId")]
    pub node_id: String,

    #[serde(rename = "nodeType")]
    pub node_type: String,

    #[serde(rename = "numInputs")]
    pub num_inputs: u32,

    #[serde(rename = "numOutputs")]
    pub num_outputs: u32,

    pub version: String,
}

impl Display for Preset {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "{}", self.info.display_name)?;
        writeln!(f, "  Author: {}", self.info.author)?;
        writeln!(f, "  Preset ID: {}", self.info.preset_id)?;
        writeln!(f, "  Product ID: {}", self.info.product_id)?;
        writeln!(f, "  Version: {}", self.version)?;
        writeln!(f, "  Nodes: {}", self.audio_graph.nodes.len())?;

        for node in &self.audio_graph.nodes {
            writeln!(
                f,
                "    {} — {} ({})",
                node.node_id, node.fender_id, node.node_type
            )?;
        }

        writeln!(f, "  Connections: {}", self.audio_graph.connections.len())?;
        for connection in &self.audio_graph.connections {
            writeln!(
                f,
                "    {}[{}] -> {}[{}]",
                connection.output.node_id,
                connection.output.index,
                connection.input.node_id,
                connection.input.index
            )?;
        }

        Ok(())
    }
}

impl Preset {
    pub fn from_data<S: AsRef<str>>(data: S) -> Result<Self, DynamicError> {
        Ok(serde_json::from_str(data.as_ref())?)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AudioGraph {
    pub connections: Vec<Connection>,
    pub nodes: Vec<DspNode>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Connection {
    pub input: ConnectionPoint,
    pub output: ConnectionPoint,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ConnectionPoint {
    pub index: u32,

    #[serde(rename = "nodeId")]
    pub node_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DspNode {
    #[serde(rename = "FenderId")]
    pub fender_id: String,

    #[serde(rename = "dspUnitParameters")]
    pub dsp_unit_parameters: HashMap<String, Value>,

    #[serde(rename = "nodeId")]
    pub node_id: String,

    #[serde(rename = "nodeType")]
    pub node_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PresetInfo {
    pub author: String,
    pub bpm: u32,
    pub created_at: u64,

    #[serde(rename = "displayName")]
    pub display_name: String,

    pub is_factory_default: bool,
    pub preset_id: String,
    pub product_id: String,
    pub source_id: String,
    pub timestamp: u64,
}
