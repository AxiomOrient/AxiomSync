/// Production default loopback endpoint for runtime LLM requests.
pub(crate) const DEFAULT_LLM_ENDPOINT: &str = "http://127.0.0.1:11434/api/chat";

/// Production default model used by runtime LLM requests.
pub(crate) const DEFAULT_LLM_MODEL: &str = "qwen2.5:7b-instruct";

/// Fixture model used across tests that construct request/contract stubs.
#[cfg(test)]
pub(crate) const TEST_OM_LLM_MODEL: &str = "qwen2.5:7b";
