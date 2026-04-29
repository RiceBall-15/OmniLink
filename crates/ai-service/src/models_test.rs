use crate::models::{ChatRequest, CreateAssistantRequest};
use validator::Validate;
use uuid::Uuid;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_request_validation_valid() {
        let request = ChatRequest {
            conversation_id: Uuid::new_v4(),
            assistant_id: Uuid::new_v4(),
            message: "Hello, AI!".to_string(),
            stream: Some(true),
            temperature: Some(0.7),
            max_tokens: Some(2048),
            model_id: None,
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_chat_request_validation_empty_message() {
        let request = ChatRequest {
            conversation_id: Uuid::new_v4(),
            assistant_id: Uuid::new_v4(),
            message: "".to_string(),
            stream: None,
            temperature: None,
            max_tokens: None,
            model_id: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_create_assistant_request_validation_valid() {
        let request = CreateAssistantRequest {
            name: "Test Assistant".to_string(),
            description: Some("A test assistant".to_string()),
            model_id: "gpt-3.5-turbo".to_string(),
            system_prompt: Some("You are a helpful assistant".to_string()),
            temperature: Some(0.7),
            max_tokens: Some(2048),
        };

        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_create_assistant_request_validation_empty_name() {
        let request = CreateAssistantRequest {
            name: "".to_string(),
            description: None,
            model_id: "gpt-3.5-turbo".to_string(),
            system_prompt: None,
            temperature: None,
            max_tokens: None,
        };

        assert!(request.validate().is_err());
    }

    #[test]
    fn test_create_assistant_request_validation_long_name() {
        let request = CreateAssistantRequest {
            name: "A".repeat(101),
            description: None,
            model_id: "gpt-3.5-turbo".to_string(),
            system_prompt: None,
            temperature: None,
            max_tokens: None,
        };

        assert!(request.validate().is_err());
    }
}