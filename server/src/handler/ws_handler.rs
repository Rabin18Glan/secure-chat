use crate::dto::ws_dto::{WsIncomingMessage, WsOutgoingMessage};
use crate::state::auth_state::{AuthStore, AuthUser};
use crate::state::ws_state::{LiveSession, SessionParticipant, WaitingParticipant, WsState};
use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        Query, State,
    },
    response::IntoResponse,
};
use axum::{
    http::{HeaderMap, StatusCode},
    Extension,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tracing::info;

pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Extension(user): Extension<AuthUser>,
    Extension(auth): Extension<AuthStore>,
    headers: HeaderMap,
    State(state): State<WsState>,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let initial_user_id = params
        .get("userId")
        .or_else(|| params.get("user_id"))
        .or_else(|| params.get("id"))
        .cloned();

    if initial_user_id.as_ref().is_some_and(|id| id != &user.0) {
        return StatusCode::FORBIDDEN.into_response();
    }
    ws.max_message_size(128 * 1024)
        .max_frame_size(128 * 1024)
        .on_upgrade(move |socket| handle_socket(socket, state, Some(user.0), auth, headers))
        .into_response()
}

async fn handle_socket(
    socket: WebSocket,
    state: WsState,
    initial_user_id: Option<String>,
    auth: AuthStore,
    headers: HeaderMap,
) {
    let (mut ws_sender, mut ws_receiver) = socket.split();
    let (tx, mut rx) = mpsc::channel::<Message>(256);

    let forward_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut current_user_id = initial_user_id.clone();
    let mut current_session_id: Option<String> = None;

    while let Some(Ok(msg)) = ws_receiver.next().await {
        if auth.authenticate(&headers).await.is_none() {
            break;
        }
        match msg {
            Message::Text(text) => {
                if let Ok(incoming) = serde_json::from_str::<WsIncomingMessage>(&text) {
                    if incoming
                        .user_id
                        .as_ref()
                        .is_some_and(|id| Some(id) != current_user_id.as_ref())
                    {
                        let _ = tx.try_send(Message::Text(
                            json!({"event":"session-error","msg":"Identity mismatch"})
                                .to_string()
                                .into(),
                        ));
                        continue;
                    }
                    let event_type = incoming
                        .event
                        .as_deref()
                        .or(incoming.message_type.as_deref())
                        .unwrap_or_default();

                    match event_type {
                        /* ---------------- Direct 1-to-1 Chat ---------------- */
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

                            if let Some(uid) =
                                user_id.filter(|id| Some(id) == current_user_id.as_ref())
                            {
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
                                        let _ = target_tx.try_send(Message::Text(json_str.into()));
                                    }
                                }
                            }
                        }

                        /* ---------------- Session & Waiting Room ---------------- */
                        "session-knock" => {
                            if let (Some(sess_id), Some(uid)) = (
                                incoming.session_id.clone(),
                                incoming.user_id.clone().or_else(|| current_user_id.clone()),
                            ) {
                                if current_session_id.as_ref().is_some_and(|id| id != &sess_id) {
                                    continue;
                                }
                                current_session_id = Some(sess_id.clone());
                                let username =
                                    incoming.username.unwrap_or_else(|| "User".to_string());
                                let avatar = incoming.avatar.unwrap_or_default();

                                if let Ok(Some(db_session)) =
                                    state.session_service.get_session(&sess_id).await
                                {
                                    let mut sessions = state.live_sessions.write().await;
                                    let live_sess =
                                        sessions.entry(sess_id.clone()).or_insert_with(|| {
                                            LiveSession {
                                                session_id: sess_id.clone(),
                                                owner_id: db_session.owner_id.clone(),
                                                admins: db_session.admins.iter().cloned().collect(),
                                                participants: HashMap::new(),
                                                waiting_room: HashMap::new(),
                                            }
                                        });

                                    let is_owner = live_sess.owner_id == uid;
                                    let is_admin = live_sess.admins.contains(&uid);

                                    if is_owner || is_admin {
                                        let role = if is_owner { "owner" } else { "admin" };
                                        live_sess.participants.insert(
                                            uid.clone(),
                                            SessionParticipant {
                                                user_id: uid.clone(),
                                                username: username.clone(),
                                                avatar: avatar.clone(),
                                                role: role.to_string(),
                                                sender: tx.clone(),
                                            },
                                        );

                                        let participants_list: Vec<_> = live_sess
                                            .participants
                                            .values()
                                            .map(|p| {
                                                json!({
                                                    "userId": p.user_id,
                                                    "username": p.username,
                                                    "avatar": p.avatar,
                                                    "role": p.role
                                                })
                                            })
                                            .collect();

                                        let waiting_list: Vec<_> = live_sess
                                            .waiting_room
                                            .values()
                                            .map(|w| {
                                                json!({
                                                    "userId": w.user_id,
                                                    "username": w.username,
                                                    "avatar": w.avatar
                                                })
                                            })
                                            .collect();

                                        let _ = tx.try_send(Message::Text(
                                            json!({
                                                "event": "session-admitted",
                                                "sessionId": sess_id,
                                                "role": role,
                                                "ownerId": live_sess.owner_id,
                                                "admins": live_sess.admins.iter().collect::<Vec<_>>(),
                                                "participants": participants_list,
                                                "waitingRoom": waiting_list,
                                            })
                                            .to_string()
                                            .into(),
                                        ));

                                        for (p_id, p) in &live_sess.participants {
                                            if p_id != &uid {
                                                let _ = p.sender.try_send(Message::Text(
                                                    json!({
                                                        "event": "session-user-joined",
                                                        "sessionId": sess_id,
                                                        "user": {
                                                            "userId": uid,
                                                            "username": username,
                                                            "avatar": avatar,
                                                            "role": role
                                                        }
                                                    })
                                                    .to_string()
                                                    .into(),
                                                ));
                                            }
                                        }
                                    } else {
                                        live_sess.waiting_room.insert(
                                            uid.clone(),
                                            WaitingParticipant {
                                                user_id: uid.clone(),
                                                username: username.clone(),
                                                avatar: avatar.clone(),
                                                sender: tx.clone(),
                                            },
                                        );

                                        let _ = tx.try_send(Message::Text(
                                            json!({
                                                "event": "session-waiting",
                                                "sessionId": sess_id,
                                                "msg": "Waiting for host approval to enter the session..."
                                            })
                                            .to_string()
                                            .into(),
                                        ));

                                        for p in live_sess.participants.values() {
                                            if p.role == "owner" || p.role == "admin" {
                                                let _ = p.sender.try_send(Message::Text(
                                                    json!({
                                                        "event": "session-knock-request",
                                                        "sessionId": sess_id,
                                                        "user": {
                                                            "userId": uid,
                                                            "username": username,
                                                            "avatar": avatar
                                                        }
                                                    })
                                                    .to_string()
                                                    .into(),
                                                ));
                                            }
                                        }
                                    }
                                } else {
                                    let _ = tx.try_send(Message::Text(
                                        json!({
                                            "event": "session-error",
                                            "msg": "Session not found or has already ended."
                                        })
                                        .to_string()
                                        .into(),
                                    ));
                                }
                            }
                        }

                        "session-approve-user" => {
                            if let (Some(sess_id), Some(target_uid), Some(ref uid)) = (
                                incoming.session_id.clone(),
                                incoming.target_user_id.clone(),
                                &current_user_id,
                            ) {
                                let mut sessions = state.live_sessions.write().await;
                                if let Some(live_sess) = sessions.get_mut(&sess_id) {
                                    if !owns_connection(live_sess, uid, &tx) {
                                        continue;
                                    }
                                    let is_admin = live_sess.owner_id == *uid
                                        || live_sess.admins.contains(uid);
                                    if is_admin {
                                        if let Some(waiting) =
                                            live_sess.waiting_room.remove(&target_uid)
                                        {
                                            live_sess.participants.insert(
                                                target_uid.clone(),
                                                SessionParticipant {
                                                    user_id: target_uid.clone(),
                                                    username: waiting.username.clone(),
                                                    avatar: waiting.avatar.clone(),
                                                    role: "member".to_string(),
                                                    sender: waiting.sender.clone(),
                                                },
                                            );

                                            let participants_list: Vec<_> = live_sess
                                                .participants
                                                .values()
                                                .map(|p| {
                                                    json!({
                                                        "userId": p.user_id,
                                                        "username": p.username,
                                                        "avatar": p.avatar,
                                                        "role": p.role
                                                    })
                                                })
                                                .collect();

                                            let _ = waiting.sender.try_send(Message::Text(
                                                json!({
                                                    "event": "session-admitted",
                                                    "sessionId": sess_id,
                                                    "role": "member",
                                                    "ownerId": live_sess.owner_id,
                                                    "admins": live_sess.admins.iter().collect::<Vec<_>>(),
                                                    "participants": participants_list,
                                                    "waitingRoom": []
                                                })
                                                .to_string()
                                                .into(),
                                            ));

                                            for (p_id, p) in &live_sess.participants {
                                                if p_id != &target_uid {
                                                    let _ = p.sender.try_send(Message::Text(
                                                        json!({
                                                            "event": "session-user-joined",
                                                            "sessionId": sess_id,
                                                            "user": {
                                                                "userId": target_uid,
                                                                "username": waiting.username,
                                                                "avatar": waiting.avatar,
                                                                "role": "member"
                                                            }
                                                        })
                                                        .to_string()
                                                        .into(),
                                                    ));
                                                }
                                            }

                                            let waiting_list: Vec<_> = live_sess
                                                .waiting_room
                                                .values()
                                                .map(|w| {
                                                    json!({
                                                        "userId": w.user_id,
                                                        "username": w.username,
                                                        "avatar": w.avatar
                                                    })
                                                })
                                                .collect();

                                            for p in live_sess.participants.values() {
                                                if p.role == "owner" || p.role == "admin" {
                                                    let _ = p.sender.try_send(Message::Text(
                                                        json!({
                                                            "event": "session-waiting-list",
                                                            "sessionId": sess_id,
                                                            "waitingRoom": waiting_list
                                                        })
                                                        .to_string()
                                                        .into(),
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        "session-reject-user" => {
                            if let (Some(sess_id), Some(target_uid), Some(ref uid)) = (
                                incoming.session_id.clone(),
                                incoming.target_user_id.clone(),
                                &current_user_id,
                            ) {
                                let mut sessions = state.live_sessions.write().await;
                                if let Some(live_sess) = sessions.get_mut(&sess_id) {
                                    if !owns_connection(live_sess, uid, &tx) {
                                        continue;
                                    }
                                    let is_admin = live_sess.owner_id == *uid
                                        || live_sess.admins.contains(uid);
                                    if is_admin {
                                        if let Some(waiting) =
                                            live_sess.waiting_room.remove(&target_uid)
                                        {
                                            let _ = waiting.sender.try_send(Message::Text(
                                                json!({
                                                    "event": "session-denied",
                                                    "sessionId": sess_id,
                                                    "msg": "Join request was declined by the session host."
                                                })
                                                .to_string()
                                                .into(),
                                            ));

                                            let waiting_list: Vec<_> = live_sess
                                                .waiting_room
                                                .values()
                                                .map(|w| {
                                                    json!({
                                                        "userId": w.user_id,
                                                        "username": w.username,
                                                        "avatar": w.avatar
                                                    })
                                                })
                                                .collect();

                                            for p in live_sess.participants.values() {
                                                if p.role == "owner" || p.role == "admin" {
                                                    let _ = p.sender.try_send(Message::Text(
                                                        json!({
                                                            "event": "session-waiting-list",
                                                            "sessionId": sess_id,
                                                            "waitingRoom": waiting_list
                                                        })
                                                        .to_string()
                                                        .into(),
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        /* ---------------- Role Management (Super Admin & Admins) ---------------- */
                        "session-promote-admin" => {
                            if let (Some(sess_id), Some(target_uid), Some(ref uid)) = (
                                incoming.session_id.clone(),
                                incoming.target_user_id.clone(),
                                &current_user_id,
                            ) {
                                let mut sessions = state.live_sessions.write().await;
                                if let Some(live_sess) = sessions.get_mut(&sess_id) {
                                    if !owns_connection(live_sess, uid, &tx) {
                                        continue;
                                    }
                                    let is_authorized = live_sess.owner_id == *uid
                                        || live_sess.admins.contains(uid);
                                    if is_authorized
                                        && live_sess.participants.contains_key(&target_uid)
                                    {
                                        live_sess.admins.insert(target_uid.clone());
                                        if let Some(p) = live_sess.participants.get_mut(&target_uid)
                                        {
                                            if p.role != "owner" {
                                                p.role = "admin".to_string();
                                            }
                                        }

                                        let admin_list: Vec<_> =
                                            live_sess.admins.iter().cloned().collect();
                                        for p in live_sess.participants.values() {
                                            let _ = p.sender.try_send(Message::Text(
                                                json!({
                                                    "event": "session-roles-updated",
                                                    "sessionId": sess_id,
                                                    "admins": admin_list,
                                                    "ownerId": live_sess.owner_id
                                                })
                                                .to_string()
                                                .into(),
                                            ));
                                        }

                                        let sess_clone = sess_id.clone();
                                        let req_uid = uid.clone();
                                        let targ_uid = target_uid.clone();
                                        let s_service = state.session_service.clone();
                                        tokio::spawn(async move {
                                            let _ = s_service
                                                .promote_admin(&sess_clone, &req_uid, &targ_uid)
                                                .await;
                                        });
                                    }
                                }
                            }
                        }

                        "session-demote-admin" => {
                            if let (Some(sess_id), Some(target_uid), Some(ref uid)) = (
                                incoming.session_id.clone(),
                                incoming.target_user_id.clone(),
                                &current_user_id,
                            ) {
                                let mut sessions = state.live_sessions.write().await;
                                if let Some(live_sess) = sessions.get_mut(&sess_id) {
                                    if !owns_connection(live_sess, uid, &tx) {
                                        continue;
                                    }
                                    if live_sess.owner_id != target_uid {
                                        let is_authorized = live_sess.owner_id == *uid
                                            || live_sess.admins.contains(uid);
                                        if is_authorized {
                                            live_sess.admins.remove(&target_uid);
                                            if let Some(p) =
                                                live_sess.participants.get_mut(&target_uid)
                                            {
                                                p.role = "member".to_string();
                                            }

                                            let admin_list: Vec<_> =
                                                live_sess.admins.iter().cloned().collect();
                                            for p in live_sess.participants.values() {
                                                let _ = p.sender.try_send(Message::Text(
                                                    json!({
                                                        "event": "session-roles-updated",
                                                        "sessionId": sess_id,
                                                        "admins": admin_list,
                                                        "ownerId": live_sess.owner_id
                                                    })
                                                    .to_string()
                                                    .into(),
                                                ));
                                            }

                                            let sess_clone = sess_id.clone();
                                            let req_uid = uid.clone();
                                            let targ_uid = target_uid.clone();
                                            let s_service = state.session_service.clone();
                                            tokio::spawn(async move {
                                                let _ = s_service
                                                    .demote_admin(&sess_clone, &req_uid, &targ_uid)
                                                    .await;
                                            });
                                        }
                                    }
                                }
                            }
                        }

                        "session-remove-user" => {
                            if let (Some(sess_id), Some(target_uid), Some(ref uid)) = (
                                incoming.session_id.clone(),
                                incoming.target_user_id.clone(),
                                &current_user_id,
                            ) {
                                let mut sessions = state.live_sessions.write().await;
                                if let Some(live_sess) = sessions.get_mut(&sess_id) {
                                    if !owns_connection(live_sess, uid, &tx) {
                                        continue;
                                    }
                                    if live_sess.owner_id != target_uid {
                                        let is_authorized = live_sess.owner_id == *uid
                                            || live_sess.admins.contains(uid);
                                        if is_authorized {
                                            if let Some(removed_p) =
                                                live_sess.participants.remove(&target_uid)
                                            {
                                                live_sess.admins.remove(&target_uid);

                                                let _ = removed_p.sender.try_send(Message::Text(
                                                    json!({
                                                        "event": "session-kicked",
                                                        "sessionId": sess_id,
                                                        "msg": "You have been removed from the session by an admin."
                                                    })
                                                    .to_string()
                                                    .into(),
                                                ));

                                                for p in live_sess.participants.values() {
                                                    let _ = p.sender.try_send(Message::Text(
                                                        json!({
                                                            "event": "session-user-left",
                                                            "sessionId": sess_id,
                                                            "userId": target_uid
                                                        })
                                                        .to_string()
                                                        .into(),
                                                    ));
                                                }

                                                let sess_clone = sess_id.clone();
                                                let req_uid = uid.clone();
                                                let targ_uid = target_uid.clone();
                                                let s_service = state.session_service.clone();
                                                tokio::spawn(async move {
                                                    let _ = s_service
                                                        .remove_member(
                                                            &sess_clone,
                                                            &req_uid,
                                                            &targ_uid,
                                                        )
                                                        .await;
                                                });
                                            }
                                        }
                                    }
                                }
                            }
                        }

                        /* ---------------- Session Leave & Terminate ---------------- */
                        "session-leave" => {
                            if let (Some(sess_id), Some(ref uid)) = (
                                incoming
                                    .session_id
                                    .clone()
                                    .or_else(|| current_session_id.clone()),
                                &current_user_id,
                            ) {
                                let mut sessions = state.live_sessions.write().await;
                                let mut should_terminate = false;
                                if let Some(live_sess) = sessions.get_mut(&sess_id) {
                                    if !owns_connection(live_sess, uid, &tx)
                                        && !live_sess
                                            .waiting_room
                                            .get(uid)
                                            .is_some_and(|p| p.sender.same_channel(&tx))
                                    {
                                        continue;
                                    }
                                    let was_participant = live_sess.participants.contains_key(uid);
                                    live_sess.waiting_room.remove(uid);
                                    broadcast_waiting_list(live_sess);
                                    if live_sess.participants.remove(uid).is_some() {
                                        for p in live_sess.participants.values() {
                                            let _ = p.sender.try_send(Message::Text(
                                                json!({
                                                    "event": "session-user-left",
                                                    "sessionId": sess_id,
                                                    "userId": uid
                                                })
                                                .to_string()
                                                .into(),
                                            ));
                                        }
                                    }
                                    if was_participant && live_sess.participants.is_empty() {
                                        should_terminate = true;
                                        for w in live_sess.waiting_room.values() {
                                            let _ = w.sender.try_send(Message::Text(
                                                json!({
                                                    "event": "session-terminated",
                                                    "sessionId": sess_id,
                                                    "msg": "The session has ended because all participants have left."
                                                })
                                                .to_string()
                                                .into(),
                                            ));
                                        }
                                    }
                                }
                                if should_terminate {
                                    info!("Session {} has no active participants. Terminating session.", sess_id);
                                    sessions.remove(&sess_id);
                                    let s_service = state.session_service.clone();
                                    let s_id = sess_id.clone();
                                    tokio::spawn(async move {
                                        let _ = s_service.cleanup_empty_session(&s_id).await;
                                    });
                                }
                                if current_session_id.as_deref() == Some(&sess_id) {
                                    current_session_id = None;
                                }
                            }
                        }

                        "session-end" | "session-terminate" => {
                            if let (Some(sess_id), Some(ref uid)) = (
                                incoming
                                    .session_id
                                    .clone()
                                    .or_else(|| current_session_id.clone()),
                                &current_user_id,
                            ) {
                                let mut sessions = state.live_sessions.write().await;
                                let is_admin = if let Some(live_sess) = sessions.get(&sess_id) {
                                    owns_connection(live_sess, uid, &tx)
                                        && (live_sess.owner_id == *uid
                                            || live_sess.admins.contains(uid))
                                } else {
                                    false
                                };

                                if is_admin {
                                    if let Some(live_sess) = sessions.remove(&sess_id) {
                                        info!(
                                            "Session {} terminated by admin/owner {}",
                                            sess_id, uid
                                        );
                                        let terminate_msg = json!({
                                            "event": "session-terminated",
                                            "sessionId": sess_id,
                                            "msg": "The call was ended for everyone by the session admin."
                                        })
                                        .to_string();

                                        for p in live_sess.participants.values() {
                                            let _ = p.sender.try_send(Message::Text(
                                                terminate_msg.clone().into(),
                                            ));
                                        }
                                        for w in live_sess.waiting_room.values() {
                                            let _ = w.sender.try_send(Message::Text(
                                                terminate_msg.clone().into(),
                                            ));
                                        }

                                        let s_service = state.session_service.clone();
                                        let s_id = sess_id.clone();
                                        tokio::spawn(async move {
                                            let _ = s_service.cleanup_empty_session(&s_id).await;
                                        });
                                    }
                                    if current_session_id.as_deref() == Some(&sess_id) {
                                        current_session_id = None;
                                    }
                                } else {
                                    let _ = tx.try_send(Message::Text(
                                        json!({
                                            "event": "session-error",
                                            "msg": "Only admins can terminate the session for everyone."
                                        })
                                        .to_string()
                                        .into(),
                                    ));
                                }
                            }
                        }

                        /* ---------------- End-to-End Encrypted (E2EE) Chat ---------------- */
                        "session-chat-message" => {
                            if let (Some(sess_id), Some(ciphertext), Some(iv), Some(ref uid)) = (
                                incoming.session_id.clone(),
                                incoming.ciphertext,
                                incoming.iv,
                                &current_user_id,
                            ) {
                                let sessions = state.live_sessions.read().await;
                                if let Some(live_sess) = sessions.get(&sess_id) {
                                    if !owns_connection(live_sess, uid, &tx) {
                                        continue;
                                    }
                                    if let Some(sender_p) = live_sess.participants.get(uid) {
                                        let payload = json!({
                                            "event": "session-chat-message",
                                            "sessionId": sess_id,
                                            "ciphertext": ciphertext,
                                            "iv": iv,
                                            "senderId": uid,
                                            "senderName": sender_p.username,
                                            "senderAvatar": sender_p.avatar,
                                            "timestamp": chrono::Utc::now().to_rfc3339()
                                        })
                                        .to_string();

                                        for p in live_sess.participants.values() {
                                            let _ = p
                                                .sender
                                                .try_send(Message::Text(payload.clone().into()));
                                        }
                                    }
                                }
                            }
                        }

                        /* ---------------- WebRTC Video/Audio Calling ---------------- */
                        "webrtc-offer" | "webrtc-answer" | "webrtc-ice-candidate" => {
                            if let (Some(sess_id), Some(to_uid), Some(ref uid)) = (
                                incoming.session_id.clone(),
                                incoming.to.clone(),
                                &current_user_id,
                            ) {
                                let sessions = state.live_sessions.read().await;
                                if let Some(live_sess) = sessions.get(&sess_id) {
                                    if !owns_connection(live_sess, uid, &tx) {
                                        continue;
                                    }
                                    if let Some(target_p) = live_sess.participants.get(&to_uid) {
                                        let forward_payload = json!({
                                            "event": event_type,
                                            "sessionId": sess_id,
                                            "from": uid,
                                            "sdp": incoming.sdp,
                                            "candidate": incoming.candidate
                                        })
                                        .to_string();

                                        let _ = target_p
                                            .sender
                                            .try_send(Message::Text(forward_payload.into()));
                                    }
                                }
                            }
                        }

                        "webrtc-media-toggle" => {
                            if let (Some(sess_id), Some(ref uid)) =
                                (incoming.session_id.clone(), &current_user_id)
                            {
                                let sessions = state.live_sessions.read().await;
                                if let Some(live_sess) = sessions.get(&sess_id) {
                                    if !owns_connection(live_sess, uid, &tx) {
                                        continue;
                                    }
                                    let forward_payload = json!({
                                        "event": "webrtc-media-toggle",
                                        "sessionId": sess_id,
                                        "from": uid,
                                        "data": incoming.data
                                    })
                                    .to_string();

                                    for (p_id, p) in &live_sess.participants {
                                        if p_id != uid {
                                            let _ = p.sender.try_send(Message::Text(
                                                forward_payload.clone().into(),
                                            ));
                                        }
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

    // Cleanup on disconnect
    if let (Some(ref sess_id), Some(ref uid)) = (&current_session_id, &current_user_id) {
        let mut sessions = state.live_sessions.write().await;
        let mut should_terminate = false;
        if let Some(live_sess) = sessions.get_mut(sess_id) {
            let was_participant = owns_connection(live_sess, uid, &tx);
            if live_sess
                .waiting_room
                .get(uid)
                .is_some_and(|p| p.sender.same_channel(&tx))
            {
                live_sess.waiting_room.remove(uid);
                broadcast_waiting_list(live_sess);
            }
            if was_participant && live_sess.participants.remove(uid).is_some() {
                for p in live_sess.participants.values() {
                    let _ = p.sender.try_send(Message::Text(
                        json!({
                            "event": "session-user-left",
                            "sessionId": sess_id,
                            "userId": uid
                        })
                        .to_string()
                        .into(),
                    ));
                }
            }
            // If all active participants have left, terminate the session
            if was_participant && live_sess.participants.is_empty() {
                should_terminate = true;
                for w in live_sess.waiting_room.values() {
                    let _ = w.sender.try_send(Message::Text(
                        json!({
                            "event": "session-terminated",
                            "sessionId": sess_id,
                            "msg": "The session has ended because all participants have left."
                        })
                        .to_string()
                        .into(),
                    ));
                }
            }
        }
        if should_terminate {
            info!(
                "Session {} has no active participants on disconnect. Terminating session.",
                sess_id
            );
            sessions.remove(sess_id);
            let s_service = state.session_service.clone();
            let s_id = sess_id.clone();
            tokio::spawn(async move {
                let _ = s_service.cleanup_empty_session(&s_id).await;
            });
        }
    }

    if let Some(ref uid) = current_user_id {
        info!("User {} disconnected", uid);
        let mut online = state.online_users.write().await;
        if online
            .get(uid)
            .is_some_and(|sender| sender.same_channel(&tx))
        {
            online.remove(uid);
        }
    }
    forward_task.abort();
}

fn owns_connection(session: &LiveSession, user_id: &str, sender: &mpsc::Sender<Message>) -> bool {
    session
        .participants
        .get(user_id)
        .is_some_and(|p| p.sender.same_channel(sender))
}

fn broadcast_waiting_list(session: &LiveSession) {
    let waiting: Vec<_> = session
        .waiting_room
        .values()
        .map(|w| json!({"userId": w.user_id, "username": w.username, "avatar": w.avatar}))
        .collect();
    let message = Message::Text(json!({"event":"session-waiting-list", "sessionId":session.session_id, "waitingRoom":waiting}).to_string().into());
    for participant in session
        .participants
        .values()
        .filter(|p| p.role == "owner" || p.role == "admin")
    {
        let _ = participant.sender.try_send(message.clone());
    }
}
