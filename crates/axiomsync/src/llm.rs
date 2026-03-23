use std::sync::Arc;

use crate::ports::{LlmExtractionPort, SharedLlmExtractionPort};

#[derive(Debug, Default)]
pub struct MockLlmClient;

impl LlmExtractionPort for MockLlmClient {}

pub fn default_llm_client() -> SharedLlmExtractionPort {
    Arc::new(MockLlmClient)
}
