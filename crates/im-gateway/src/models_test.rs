#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_message_serialization() {
        let message = crate::models::WSMessage {
            message_type: crate::models::WSMessageType::Message,
            conversation_id: Some(uuid::Uuid::new_v4()),
            message_id: Some(uuid::Uuid::new_v4()),
            sender_id: Some(uuid::Uuid::new_v4()),
            content: Some("Hello".to_string()),
            timestamp: Some(chrono::Utc::now().timestamp()),
            data: None,
        };

        let json = serde_json::to_string(&message).unwrap();
        println!("WSMessage JSON: {}", json);
        assert!(json.contains("\"message_type\":\"message\""));
    }

    #[test]
    fn test_send_message_request_validation() {
        let request = crate::models::SendMessageRequest {
            conversation_id: uuid::Uuid::new_v4(),
            content: "Hello, World!".to_string(),
            message_type: Some("text".to_string()),
            reply_to: None,
            metadata: None,
        };

        let validation = request.validate();
        assert!(validation.is_ok());
    }

    #[test]
    fn test_send_message_request_empty_content() {
        let request = crate::models::SendMessageRequest {
            conversation_id: uuid::Uuid::new_v4(),
            content: "".to_string(),
            message_type: Some("text".to_string()),
            reply_to: None,
            metadata: None,
        };

        let validation = request.validate();
        assert!(validation.is_err());
    }

    #[test]
    fn test_create_conversation_request_validation() {
        let request = crate::models::CreateConversationRequest {
            name: "Test Conversation".to_string(),
            participant_ids: vec![uuid::Uuid::new_v4()],
            is_group: true,
            description: Some("Test description".to_string()),
        };

        let validation = request.validate();
        assert!(validation.is_ok());
    }

    #[test]
    fn test_ws_message_type_serialization() {
        let types = vec![
            crate::models::WSMessageType::Connect,
            crate::models::WSMessageType::Connected,
            crate::models::WSMessageType::Message,
            crate::models::WSMessageType::MessageRead,
            crate::models::WSMessageType::Online,
            crate::models::WSMessageType::Typing,
            crate::models::WSMessageType::Ping,
        ];

        for msg_type in types {
            let json = serde_json::to_string(&msg_type).unwrap();
            println!("MessageType JSON: {}", json);
            let deserialized: crate::models::WSMessageType = serde_json::from_str(&json).unwrap();
            assert_eq!(format!("{:?}", msg_type), format!("{:?}", deserialized));
        }
    }
}