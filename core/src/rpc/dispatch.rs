use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};
use std::time::Duration;

use serde_json::Value;
use tokio::sync::{mpsc, oneshot};

use crate::rpc::jsonrpc::{
    IncomingMessage, JsonRpcError, JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, RequestId,
};
use crate::rpc::transport::Transport;
use crate::{CoreError, Result};

const RECV_POLL_INTERVAL: Duration = Duration::from_millis(10);

pub struct RpcConnection;

pub type InboundRequests = mpsc::Receiver<InboundMessage>;

#[derive(Debug, Clone, PartialEq)]
pub enum InboundMessage {
    Request(JsonRpcRequest),
    Notification(JsonRpcNotification),
}

#[derive(Clone)]
pub struct RpcHandle {
    command_tx: mpsc::Sender<RpcCommand>,
    next_id: Arc<AtomicI64>,
}

enum RpcCommand {
    Request {
        request: JsonRpcRequest,
        response_tx: oneshot::Sender<Result<Value>>,
    },
    Notify(JsonRpcNotification),
    Respond(JsonRpcResponse),
    RemovePending(RequestId),
    Close(oneshot::Sender<Result<()>>),
}

impl RpcConnection {
    pub fn start(mut transport: Box<dyn Transport>) -> (RpcHandle, InboundRequests) {
        let (command_tx, mut command_rx) = mpsc::channel(64);
        let (inbound_tx, inbound_rx) = mpsc::channel(64);

        tokio::spawn(async move {
            let mut pending = HashMap::new();
            let mut closed = false;
            while !closed {
                while let Ok(command) = command_rx.try_recv() {
                    closed = handle_command(command, &mut transport, &mut pending).await;
                    if closed {
                        break;
                    }
                }
                if closed {
                    break;
                }

                match tokio::time::timeout(RECV_POLL_INTERVAL, transport.recv()).await {
                    Ok(Ok(message)) => route_incoming(message, &inbound_tx, &mut pending).await,
                    Ok(Err(err)) => {
                        fail_pending(&mut pending, err);
                        closed = true;
                    }
                    Err(_) => {
                        if command_rx.is_closed() {
                            fail_pending(&mut pending, CoreError::TransportClosed);
                            closed = true;
                        }
                    }
                }
            }

            let _ = transport.close().await;
            fail_pending(&mut pending, CoreError::TransportClosed);
        });

        (
            RpcHandle {
                command_tx,
                next_id: Arc::new(AtomicI64::new(1)),
            },
            inbound_rx,
        )
    }
}

impl RpcHandle {
    pub async fn request(
        &self,
        method: &str,
        params: Option<Value>,
        timeout: Duration,
    ) -> Result<Value> {
        let id = RequestId::Number(self.next_id.fetch_add(1, Ordering::Relaxed));
        let request = JsonRpcRequest::new(id.clone(), method, params);
        let (response_tx, response_rx) = oneshot::channel();
        self.command_tx
            .send(RpcCommand::Request {
                request,
                response_tx,
            })
            .await
            .map_err(|_| CoreError::TransportClosed)?;

        match tokio::time::timeout(timeout, response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(CoreError::TransportClosed),
            Err(_) => {
                let _ = self.command_tx.send(RpcCommand::RemovePending(id)).await;
                Err(CoreError::Timeout {
                    method: method.to_string(),
                })
            }
        }
    }

    pub async fn notify(&self, method: &str, params: Option<Value>) -> Result<()> {
        self.command_tx
            .send(RpcCommand::Notify(JsonRpcNotification::new(method, params)))
            .await
            .map_err(|_| CoreError::TransportClosed)
    }

    pub async fn respond(
        &self,
        id: RequestId,
        result: std::result::Result<Value, JsonRpcError>,
    ) -> Result<()> {
        let response = match result {
            Ok(value) => JsonRpcResponse::success(id, value),
            Err(error) => JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(error),
            },
        };
        self.command_tx
            .send(RpcCommand::Respond(response))
            .await
            .map_err(|_| CoreError::TransportClosed)
    }

    pub async fn close(self) -> Result<()> {
        let (close_tx, close_rx) = oneshot::channel();
        self.command_tx
            .send(RpcCommand::Close(close_tx))
            .await
            .map_err(|_| CoreError::TransportClosed)?;
        close_rx.await.map_err(|_| CoreError::TransportClosed)?
    }
}

async fn handle_command(
    command: RpcCommand,
    transport: &mut Box<dyn Transport>,
    pending: &mut HashMap<RequestId, oneshot::Sender<Result<Value>>>,
) -> bool {
    match command {
        RpcCommand::Request {
            request,
            response_tx,
        } => {
            let id = request.id.clone();
            match transport.send_request(&request).await {
                Ok(()) => {
                    pending.insert(id, response_tx);
                }
                Err(err) => {
                    let _ = response_tx.send(Err(err));
                }
            }
            false
        }
        RpcCommand::Notify(note) => {
            let _ = transport.send_notification(&note).await;
            false
        }
        RpcCommand::Respond(resp) => {
            let _ = transport.send_response(&resp).await;
            false
        }
        RpcCommand::RemovePending(id) => {
            pending.remove(&id);
            false
        }
        RpcCommand::Close(done) => {
            let result = transport.close().await;
            let _ = done.send(result);
            true
        }
    }
}

async fn route_incoming(
    message: IncomingMessage,
    inbound_tx: &mpsc::Sender<InboundMessage>,
    pending: &mut HashMap<RequestId, oneshot::Sender<Result<Value>>>,
) {
    match message {
        IncomingMessage::Response(response) => {
            if let Some(tx) = pending.remove(&response.id) {
                let result = if let Some(error) = response.error {
                    Err(CoreError::JsonRpc {
                        code: error.code,
                        message: error.message,
                    })
                } else {
                    response.result.ok_or_else(|| {
                        CoreError::Protocol("response missing result and error".to_string())
                    })
                };
                let _ = tx.send(result);
            }
        }
        IncomingMessage::Request(request) => {
            let _ = inbound_tx.send(InboundMessage::Request(request)).await;
        }
        IncomingMessage::Notification(note) => {
            let _ = inbound_tx.send(InboundMessage::Notification(note)).await;
        }
    }
}

fn fail_pending(
    pending: &mut HashMap<RequestId, oneshot::Sender<Result<Value>>>,
    error: CoreError,
) {
    let message = error.to_string();
    for (_, tx) in pending.drain() {
        let _ = tx.send(Err(CoreError::Protocol(message.clone())));
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::sync::Arc;

    use async_trait::async_trait;
    use serde_json::json;
    use tokio::sync::Mutex;

    use super::*;

    struct DispatchMockTransport {
        incoming: VecDeque<IncomingMessage>,
        sent: Arc<Mutex<Vec<JsonRpcRequest>>>,
        recv_delay: Duration,
    }

    #[async_trait]
    impl Transport for DispatchMockTransport {
        async fn send_request(&mut self, req: &JsonRpcRequest) -> Result<()> {
            self.sent.lock().await.push(req.clone());
            Ok(())
        }

        async fn send_notification(&mut self, _note: &JsonRpcNotification) -> Result<()> {
            Ok(())
        }

        async fn send_response(&mut self, _resp: &JsonRpcResponse) -> Result<()> {
            Ok(())
        }

        async fn recv(&mut self) -> Result<IncomingMessage> {
            tokio::time::sleep(self.recv_delay).await;
            self.incoming.pop_front().ok_or(CoreError::TransportClosed)
        }

        async fn close(&mut self) -> Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn concurrent_requests_correlate() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let transport = DispatchMockTransport {
            incoming: VecDeque::from(vec![
                IncomingMessage::Response(JsonRpcResponse::success(
                    RequestId::Number(2),
                    json!("b"),
                )),
                IncomingMessage::Response(JsonRpcResponse::success(
                    RequestId::Number(1),
                    json!("a"),
                )),
            ]),
            sent: Arc::clone(&sent),
            recv_delay: Duration::from_millis(1),
        };
        let (handle, _inbound) = RpcConnection::start(Box::new(transport));

        let (a, b) = tokio::join!(
            handle.request("a", None, Duration::from_secs(1)),
            handle.request("b", None, Duration::from_secs(1))
        );

        assert_eq!(a.unwrap(), json!("a"));
        assert_eq!(b.unwrap(), json!("b"));
        assert_eq!(sent.lock().await.len(), 2);
    }

    #[tokio::test]
    async fn inbound_request_delivered_while_request_pending() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let transport = DispatchMockTransport {
            incoming: VecDeque::from(vec![
                IncomingMessage::Request(JsonRpcRequest::new(
                    RequestId::String("srv-1".to_string()),
                    "session/request_permission",
                    Some(json!({})),
                )),
                IncomingMessage::Response(JsonRpcResponse::success(
                    RequestId::Number(1),
                    json!("ok"),
                )),
            ]),
            sent,
            recv_delay: Duration::from_millis(1),
        };
        let (handle, mut inbound) = RpcConnection::start(Box::new(transport));
        let pending = tokio::spawn(async move {
            handle
                .request("session/prompt", None, Duration::from_secs(1))
                .await
        });

        let inbound_msg = inbound.recv().await.unwrap();
        assert!(matches!(inbound_msg, InboundMessage::Request(_)));
        assert_eq!(pending.await.unwrap().unwrap(), json!("ok"));
    }

    #[tokio::test]
    async fn close_fails_pending() {
        let sent = Arc::new(Mutex::new(Vec::new()));
        let transport = DispatchMockTransport {
            incoming: VecDeque::new(),
            sent,
            recv_delay: Duration::from_secs(60),
        };
        let (handle, _inbound) = RpcConnection::start(Box::new(transport));
        let request_handle = handle.clone();
        let pending = tokio::spawn(async move {
            request_handle
                .request("slow", None, Duration::from_secs(10))
                .await
        });
        tokio::time::sleep(Duration::from_millis(20)).await;
        handle.close().await.unwrap();

        assert!(pending.await.unwrap().is_err());
    }
}
