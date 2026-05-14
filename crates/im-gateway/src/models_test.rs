use crate::models::*;
use uuid::Uuid;

#[test]
fn test_ws_message_serialize() {
    let msg = WSMessage {
        message_type: WSMessageType::Message,
        conversation_id: Some(Uuid::new_v4()),
        message_id: Some(Uuid::new_v4()),
        sender_id: Some(Uuid::new_v4()),
        content: Some("Hello".to_string()),
        timestamp: Some(1234567890),
        data: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("message"));
    assert!(json.contains("Hello"));
}

#[test]
fn test_ws_message_deserialize() {
    let msg = WSMessage {
        message_type: WSMessageType::Ping,
        conversation_id: None,
        message_id: None,
        sender_id: None,
        content: None,
        timestamp: Some(1234567890),
        data: None,
    };
    let json = serde_json::to_string(&msg).unwrap();
    let deserialized: WSMessage = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.message_type, WSMessageType::Ping);
    assert!(deserialized.content.is_none());
}

#[test]
fn test_ws_message_type_roundtrip() {
    let types = vec![
        WSMessageType::Connect,
        WSMessageType::Connected,
        WSMessageType::Disconnect,
        WSMessageType::Message,
        WSMessageType::NewMessage,
        WSMessageType::Read,
        WSMessageType::Edit,
        WSMessageType::Recall,
        WSMessageType::Ping,
        WSMessageType::Pong,
        WSMessageType::Typing,
        WSMessageType::StatusChange,
        WSMessageType::TokenRefresh,
        WSMessageType::RefreshOk,
        WSMessageType::Error,
    ];
    for msg_type in types {
        let msg = WSMessage {
            message_type: msg_type.clone(),
            conversation_id: None,
            message_id: None,
            sender_id: None,
            content: None,
            timestamp: None,
            data: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        let deserialized: WSMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.message_type, msg_type);
    }
}

#[test]
fn test_ws_connect_request_deserialize() {
    let json = r#"{"token":"abc123","conversation_id":null}"#;
    let req: WSConnectRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.token, "abc123");
    assert!(req.conversation_id.is_none());
}

#[test]
fn test_send_message_request_valid() {
    let req = SendMessageRequest {
        conversation_id: Uuid::new_v4(),
        content: "Hello".to_string(),
        message_type: Some("text".to_string()),
        reply_to: None,
        metadata: None,
    };
    assert!(req.validate().is_ok());
}

#[test]
fn test_send_message_request_empty_content() {
    let req = SendMessageRequest {
        conversation_id: Uuid::new_v4(),
        content: "".to_string(),
        message_type: None,
        reply_to: None,
        metadata: None,
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_create_conversation_request_valid() {
    let req = CreateConversationRequest {
        name: "Test Group".to_string(),
        participant_ids: vec![Uuid::new_v4(), Uuid::new_v4()],
        is_group: true,
        description: Some("A test group".to_string()),
    };
    assert!(req.validate().is_ok());
}

#[test]
fn test_create_conversation_request_empty_name() {
    let req = CreateConversationRequest {
        name: "".to_string(),
        participant_ids: vec![Uuid::new_v4()],
        is_group: false,
        description: None,
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_create_conversation_request_empty_participants() {
    let req = CreateConversationRequest {
        name: "Test".to_string(),
        participant_ids: vec![],
        is_group: false,
        description: None,
    };
    assert!(req.validate().is_err());
}

#[test]
fn test_message_history_request_deserialize() {
    let cid = Uuid::new_v4();
    let json = format!(
        r#"{{"conversation_id":"{}","limit":50,"before_message_id":null}}"#,
        cid
    );
    let req: MessageHistoryRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(req.conversation_id, cid);
    assert_eq!(req.limit, Some(50));
    assert!(req.before_message_id.is_none());
}

#[test]
fn test_online_user_info_serialize() {
    let info = OnlineUserInfo {
        user_id: Uuid::new_v4(),
        username: "testuser".to_string(),
        avatar_url: Some("https://example.com/avatar.png".to_string()),
        status: "online".to_string(),
        last_seen: 1234567890,
    };
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("online"));
    assert!(json.contains("testuser"));
}

#[test]
fn test_online_user_info_no_avatar() {
    let info = OnlineUserInfo {
        user_id: Uuid::new_v4(),
        username: "testuser".to_string(),
        avatar_url: None,
        status: "offline".to_string(),
        last_seen: 0,
    };
    let json = serde_json::to_string(&info).unwrap();
    assert!(json.contains("offline"));
    let deserialized: OnlineUserInfo = serde_json::from_str(&json).unwrap();
    assert!(deserialized.avatar_url.is_none());
}

#[test]
fn test_send_message_response_serialize() {
    let resp = SendMessageResponse {
        message_id: Uuid::new_v4(),
        conversation_id: Uuid::new_v4(),
        content: "Hello".to_string(),
        message_type: "text".to_string(),
        sender_id: Uuid::new_v4(),
        created_at: 1234567890,
    };
    let json = serde_json::to_string(&resp).unwrap();
    assert!(json.contains("text"));
    assert!(json.contains("Hello"));
}

#[test]
fn test_typing_request_deserialize() {
    let cid = Uuid::new_v4();
    let json = format!(r#"{{"conversation_id":"{}","is_typing":true}}"#, cid);
    let req: TypingRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(req.conversation_id, cid);
    assert!(req.is_typing);
}

#[test]
fn test_status_change_request_deserialize() {
    let json = r#"{"status":"away"}"#;
    let req: StatusChangeRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.status, "away");
}

#[test]
fn test_token_refresh_request_deserialize() {
    let json = r#"{"token":"new-jwt-token"}"#;
    let req: TokenRefreshRequest = serde_json::from_str(json).unwrap();
    assert_eq!(req.token, "new-jwt-token");
}

#[test]
fn test_batch_status_query_deserialize() {
    let id1 = Uuid::new_v4();
    let id2 = Uuid::new_v4();
    let json = format!(r#"{{"user_ids":["{}","{}"]}}"#, id1, id2);
    let query: BatchStatusQuery = serde_json::from_str(&json).unwrap();
    assert_eq!(query.user_ids.len(), 2);
    assert_eq!(query.user_ids[0], id1);
}

#[test]
fn test_user_status_item_serialize() {
    let item = UserStatusItem {
        user_id: Uuid::new_v4(),
        status: "online".to_string(),
        last_seen: 1234567890,
    };
    let json = serde_json::to_string(&item).unwrap();
    assert!(json.contains("online"));
}

#[test]
fn test_ws_message_with_data() {
    let data = serde_json::json!({
        "typing": true,
        "username": "testuser"
    });
    let msg = WSMessage {
        message_type: WSMessageType::Typing,
        conversation_id: Some(Uuid::new_v4()),
        message_id: None,
        sender_id: Some(Uuid::new_v4()),
        content: None,
        timestamp: Some(1234567890),
        data: Some(data),
    };
    let json = serde_json::to_string(&msg).unwrap();
    assert!(json.contains("typing"));
    let deserialized: WSMessage = serde_json::from_str(&json).unwrap();
    assert!(deserialized.data.is_some());
    assert!(deserialized.data.unwrap()["typing"].as_bool().unwrap());
}

#[test]
fn test_conversations_query_deserialize() {
    let json = r#"{"limit":20,"offset":0}"#;
    let query: ConversationsQuery = serde_json::from_str(json).unwrap();
    assert_eq!(query.limit, Some(20));
    assert_eq!(query.offset, Some(0));
}

#[test]
fn test_mark_read_request_deserialize() {
    let cid = Uuid::new_v4();
    let mid = Uuid::new_v4();
    let json = format!(r#"{{"conversation_id":"{}","message_id":"{}"}}"#, cid, mid);
    let req: MarkReadRequest = serde_json::from_str(&json).unwrap();
    assert_eq!(req.conversation_id, cid);
    assert_eq!(req.message_id, mid);
}
