use serde::Deserialize;
use serde_json::Value;

use crate::conversation::{ContentBlock, ElicitationAction, ElicitationMode};
use crate::mcp::elicitation::{elicitation_request_block, elicitation_response_block};

#[derive(Debug, Clone, Default)]
pub struct GatewayContentReducer {
    blocks: Vec<ContentBlock>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum GatewayReducerInput {
    ElicitationRequested {
        request_id: String,
        server_id: String,
        #[serde(default)]
        origin_tool_call_id: Option<String>,
        mode: ElicitationMode,
        message: String,
        #[serde(default)]
        schema: Option<Value>,
        #[serde(default)]
        url: Option<String>,
    },
    ElicitationResponse {
        request_id: String,
        action: ElicitationAction,
        #[serde(default)]
        data: Option<Value>,
    },
}

impl GatewayContentReducer {
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    pub fn apply_input(&mut self, input: &GatewayReducerInput) {
        match input {
            GatewayReducerInput::ElicitationRequested {
                request_id,
                server_id,
                origin_tool_call_id,
                mode,
                message,
                schema,
                url,
            } => self.blocks.push(elicitation_request_block(
                request_id.clone(),
                server_id.clone(),
                origin_tool_call_id.clone(),
                mode.clone(),
                message.clone(),
                schema.clone(),
                url.clone(),
            )),
            GatewayReducerInput::ElicitationResponse {
                request_id,
                action,
                data,
            } => self.blocks.push(elicitation_response_block(
                request_id.clone(),
                action.clone(),
                data.clone(),
            )),
        }
    }

    pub fn finish(self) -> Vec<ContentBlock> {
        self.blocks
    }
}
