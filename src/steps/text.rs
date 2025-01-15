use async_trait::async_trait;
use anyhow::Result;
use std::fs;
use crate::types::{ProcessError, Strategy, Config};
use crate::processor::{ProcessingStep, AsyncProcessor, format_text_data};
use crate::proto::processor::Query;

pub struct TextProcessor;

#[async_trait]
impl AsyncProcessor for TextProcessor {
    async fn process(&self, query: &mut Query, _config: &Config) -> Result<(), ProcessError> {
        let content = fs::read_to_string(&query.file_path)
            .map_err(|e| ProcessError::ExtractionFailed(e.to_string()))?;
        
        query.prompt_parts.push(format_text_data(&content));
        
        Ok(())
    }
}

impl ProcessingStep for TextProcessor {
    fn required_for(&self) -> Vec<Strategy> {
        vec![Strategy::Text]
    }

    fn name(&self) -> &'static str {
        "text_processor"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tokio;

    #[tokio::test]
    async fn test_text_processor() {
        // Create test config
        let config = Config::default();

        // Create test query
        let mut query = Query {
            file_type: "txt".to_string(),
            file_path: "test_data/text-test-1.txt".to_string(),
            strategy: Strategy::Text.to_string(),
            prompt_parts: Vec::new(),
            attachments: Vec::new(),
            system: "test".to_string(),
            prompt: String::new(),
            metadata: None,
        };

        // Process the text file
        let processor = TextProcessor;
        let result = processor.process(&mut query, &config).await;

        // Verify results
        assert!(result.is_ok(), "Processing should succeed");
        assert_eq!(query.prompt_parts.len(), 1, "Should have one prompt part");
        
        let expected_content = fs::read_to_string("test_data/text-test-1.txt").unwrap();
        let expected_formatted = format_text_data(&expected_content);
        assert_eq!(query.prompt_parts[0], expected_formatted, "Content should match");
    }
} 