use std::sync::Arc;

use reqwest::blocking::Client;
use serde_json::{Value, json};

use crate::domain::{EpisodeExtraction, VerificationExtraction};
use crate::error::{AxiomError, Result};
use crate::ports::{LlmExtractionPort, SharedLlmExtractionPort};

const EPISODE_PROMPT: &str =
    include_str!("../assets/episode_extractor_v1.md");
const VERIFICATION_PROMPT: &str =
    include_str!("../assets/verification_synthesizer_v1.md");

pub trait LlmClient: Send + Sync {
    fn extract_episode(&self, transcript: &str) -> Result<EpisodeExtraction>;
    fn synthesize_verifications(&self, transcript: &str) -> Result<Vec<VerificationExtraction>>;
}

pub type SharedLlmClient = Arc<dyn LlmClient>;

#[derive(Debug)]
pub struct DisabledLlmClient;

impl LlmClient for DisabledLlmClient {
    fn extract_episode(&self, _transcript: &str) -> Result<EpisodeExtraction> {
        Err(AxiomError::LlmUnavailable(
            "set AXIOMSYNC_LLM_BASE_URL, AXIOMSYNC_LLM_API_KEY, AXIOMSYNC_LLM_MODEL".to_string(),
        ))
    }

    fn synthesize_verifications(&self, _transcript: &str) -> Result<Vec<VerificationExtraction>> {
        Err(AxiomError::LlmUnavailable(
            "set AXIOMSYNC_LLM_BASE_URL, AXIOMSYNC_LLM_API_KEY, AXIOMSYNC_LLM_MODEL".to_string(),
        ))
    }
}

#[derive(Debug, Clone)]
pub struct OpenAiCompatibleLlmClient {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl OpenAiCompatibleLlmClient {
    pub fn from_env() -> Option<Self> {
        let base_url = std::env::var("AXIOMSYNC_LLM_BASE_URL").ok()?;
        let api_key = std::env::var("AXIOMSYNC_LLM_API_KEY").ok()?;
        let model = std::env::var("AXIOMSYNC_LLM_MODEL").ok()?;
        Some(Self {
            client: Client::new(),
            base_url,
            api_key,
            model,
        })
    }

    fn complete_json(&self, prompt: &str, transcript: &str) -> Result<Value> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        let body = json!({
            "model": self.model,
            "response_format": {"type": "json_object"},
            "messages": [
                {"role": "system", "content": prompt},
                {"role": "user", "content": transcript}
            ]
        });
        let response = self
            .client
            .post(url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .map_err(|error| AxiomError::Internal(format!("llm request failed: {error}")))?
            .error_for_status()
            .map_err(|error| AxiomError::Internal(format!("llm response failed: {error}")))?;
        let json: Value = response
            .json()
            .map_err(|error| AxiomError::Internal(format!("llm json decode failed: {error}")))?;
        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| AxiomError::Internal("missing LLM content".to_string()))?;
        Ok(serde_json::from_str(content)?)
    }
}

impl LlmClient for OpenAiCompatibleLlmClient {
    fn extract_episode(&self, transcript: &str) -> Result<EpisodeExtraction> {
        Ok(serde_json::from_value(
            self.complete_json(EPISODE_PROMPT, transcript)?,
        )?)
    }

    fn synthesize_verifications(&self, transcript: &str) -> Result<Vec<VerificationExtraction>> {
        let value = self.complete_json(VERIFICATION_PROMPT, transcript)?;
        if let Some(array) = value.as_array() {
            return Ok(serde_json::from_value(Value::Array(array.clone()))?);
        }
        if let Some(array) = value.get("verifications").and_then(Value::as_array) {
            return Ok(serde_json::from_value(Value::Array(array.clone()))?);
        }
        Err(AxiomError::Internal(
            "verification prompt did not return an array".to_string(),
        ))
    }
}

pub fn default_llm_client() -> SharedLlmExtractionPort {
    OpenAiCompatibleLlmClient::from_env()
        .map(|client| Arc::new(client) as SharedLlmExtractionPort)
        .unwrap_or_else(|| Arc::new(DisabledLlmClient) as SharedLlmExtractionPort)
}

#[derive(Debug, Clone)]
pub struct MockLlmClient {
    pub extraction: EpisodeExtraction,
    pub verifications: Vec<VerificationExtraction>,
}

impl LlmClient for MockLlmClient {
    fn extract_episode(&self, _transcript: &str) -> Result<EpisodeExtraction> {
        Ok(self.extraction.clone())
    }

    fn synthesize_verifications(&self, _transcript: &str) -> Result<Vec<VerificationExtraction>> {
        Ok(self.verifications.clone())
    }
}

macro_rules! impl_llm_port {
    ($ty:ty) => {
        impl LlmExtractionPort for $ty {
            fn extract_episode(&self, transcript: &str) -> Result<EpisodeExtraction> {
                LlmClient::extract_episode(self, transcript)
            }

            fn synthesize_verifications(
                &self,
                transcript: &str,
            ) -> Result<Vec<VerificationExtraction>> {
                LlmClient::synthesize_verifications(self, transcript)
            }
        }
    };
}

impl_llm_port!(DisabledLlmClient);
impl_llm_port!(OpenAiCompatibleLlmClient);
impl_llm_port!(MockLlmClient);
