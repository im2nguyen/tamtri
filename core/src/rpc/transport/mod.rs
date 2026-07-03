pub mod http;
pub mod stdio;

use async_trait::async_trait;

use crate::Result;
use crate::rpc::jsonrpc::{IncomingMessage, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse};

#[async_trait]
pub trait Transport: Send {
    async fn send_request(&mut self, req: &JsonRpcRequest) -> Result<()>;
    async fn send_notification(&mut self, note: &JsonRpcNotification) -> Result<()>;
    async fn send_response(&mut self, resp: &JsonRpcResponse) -> Result<()>;
    async fn recv(&mut self) -> Result<IncomingMessage>;
    async fn close(&mut self) -> Result<()>;
}
