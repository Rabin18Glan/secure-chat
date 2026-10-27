use crate::dto::ws_dto::{WsIncomingMessage, WsOutgoingMessage};
use crate::state::ws_state::WsState;
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use futures_util::{SinkExt, StreamExt};
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::info;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<WsState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let initial_user_id = params
        .get("userId")
        .or_else(|| params.get("user_id"))
        .or_else(|| params.get("id"))
        .cloned();

    ws.on_upgrade(move |socket| handle_socket(socket, state, initial_user_id))
}

async fn handle_socket(socket: WebSocket, state: WsState, initial_user_id: Option<String>) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<Message>();

    let forward_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut current_user_id = initial_user_id.clone();
    if let Some(ref uid) = current_user_id {
        info!("User {} connected via query param", uid);
        state
            .online_users
            .write()
            .await
            .insert(uid.clone(), tx.clone());
    }

    while let Some(Ok(msg)) = ws_receiver.next().await {
        match msg {
            Message::Text(text) => {
                if let Ok(incoming) = serde_json::from_str::<WsIncomingMessage>(&text) {
                    let event_type = incoming
                        .event
                        .as_deref()
                        .or(incoming.message_type.as_deref())
                        .unwrap_or_default();

                    match event_type {
                        "add-user" | "addUser" | "login" => {
                            let user_id = incoming
                                .user_id
                                .or_else(|| {
                                    incoming.data.as_ref().and_then(|d| match d {
                                        serde_json::Value::String(s) => Some(s.clone()),
                                        serde_json::Value::Object(o) => o
                                            .get("userId")
                                            .and_then(|v| v.as_str())
                                            .map(String::from),
                                        _ => None,
                                    })
                                })
                                .or(incoming.msg);

                            if let Some(uid) = user_id {
                                info!("User {} registered online", uid);
                                current_user_id = Some(uid.clone());
                                state.online_users.write().await.insert(uid, tx.clone());
                            }
                        }
                        "send-msg" | "sendMsg" | "sendMessage" => {
                            let (target_to, message_text) =
                                if let (Some(to), Some(m)) = (incoming.to, incoming.msg) {
                                    (Some(to), Some(m))
                                } else if let Some(serde_json::Value::Object(ref data_obj)) =
                                    incoming.data
                                {
                                    let to = data_obj
                                        .get("to")
                                        .and_then(|v| v.as_str())
                                        .map(String::from);
                                    let m = data_obj
                                        .get("msg")
                                        .or_else(|| data_obj.get("message"))
                                        .and_then(|v| v.as_str())
                                        .map(String::from);
                                    (to, m)
                                } else {
                                    (None, None)
                                };

                            if let (Some(to), Some(m)) = (target_to, message_text) {
                                let online = state.online_users.read().await;
                                if let Some(target_tx) = online.get(&to) {
                                    let outgoing = WsOutgoingMessage {
                                        event: "msg-recieve".to_string(),
                                        message_type: "msg-recieve".to_string(),
                                        msg: m,
                                        from: current_user_id.clone(),
                                    };
                                    if let Ok(json_str) = serde_json::to_string(&outgoing) {
                                        let _ = target_tx.send(Message::Text(json_str.into()));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Message::Close(_) => {
                break;
            }
            _ => {}
        }
    }

    if let Some(ref uid) = current_user_id {
        info!("User {} disconnected", uid);
        state.online_users.write().await.remove(uid);
    }
    forward_task.abort();
}
