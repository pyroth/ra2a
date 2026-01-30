//! Authentication and credential management for A2A clients.
//!
//! Provides credential providers and authentication helpers for
//! securing client requests to A2A agents.

use async_trait::async_trait;
use std::collections::HashMap;

use crate::error::Result;

/// Credential provider trait for obtaining authentication credentials.
#[async_trait]
pub trait CredentialProvider: Send + Sync {
    /// Gets the authentication headers for a request.
    async fn get_headers(&self) -> Result<HashMap<String, String>>;

    /// Refreshes the credentials if needed.
    async fn refresh(&self) -> Result<()> {
        Ok(())
    }
}

/// Static API key credential provider.
#[derive(Debug, Clone)]
pub struct ApiKeyCredential {
    header_name: String,
    api_key: String,
}

impl ApiKeyCredential {
    /// Creates a new API key credential.
    pub fn new(header_name: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            header_name: header_name.into(),
            api_key: api_key.into(),
        }
    }

    /// Creates an X-API-Key credential.
    pub fn x_api_key(api_key: impl Into<String>) -> Self {
        Self::new("X-API-Key", api_key)
    }
}

#[async_trait]
impl CredentialProvider for ApiKeyCredential {
    async fn get_headers(&self) -> Result<HashMap<String, String>> {
        let mut headers = HashMap::new();
        headers.insert(self.header_name.clone(), self.api_key.clone());
        Ok(headers)
    }
}

/// Bearer token credential provider.
#[derive(Debug, Clone)]
pub struct BearerTokenCredential {
    token: String,
}

impl BearerTokenCredential {
    /// Creates a new bearer token credential.
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
        }
    }
}

#[async_trait]
impl CredentialProvider for BearerTokenCredential {
    async fn get_headers(&self) -> Result<HashMap<String, String>> {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            format!("Bearer {}", self.token),
        );
        Ok(headers)
    }
}

/// Basic authentication credential provider.
#[derive(Debug, Clone)]
pub struct BasicAuthCredential {
    username: String,
    password: String,
}

impl BasicAuthCredential {
    /// Creates a new basic auth credential.
    pub fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: username.into(),
            password: password.into(),
        }
    }

    /// Encodes the credentials to base64.
    fn encode(&self) -> String {
        use base64::Engine;
        let credentials = format!("{}:{}", self.username, self.password);
        base64::engine::general_purpose::STANDARD.encode(credentials.as_bytes())
    }
}

#[async_trait]
impl CredentialProvider for BasicAuthCredential {
    async fn get_headers(&self) -> Result<HashMap<String, String>> {
        let mut headers = HashMap::new();
        headers.insert(
            "Authorization".to_string(),
            format!("Basic {}", self.encode()),
        );
        Ok(headers)
    }
}

/// No-op credential provider for unauthenticated requests.
#[derive(Debug, Clone, Default)]
pub struct NoCredential;

#[async_trait]
impl CredentialProvider for NoCredential {
    async fn get_headers(&self) -> Result<HashMap<String, String>> {
        Ok(HashMap::new())
    }
}

/// Composite credential provider that combines multiple providers.
pub struct CompositeCredential {
    providers: Vec<Box<dyn CredentialProvider>>,
}

impl CompositeCredential {
    /// Creates a new composite credential.
    pub fn new() -> Self {
        Self {
            providers: Vec::new(),
        }
    }

    /// Adds a credential provider.
    pub fn add<C: CredentialProvider + 'static>(mut self, provider: C) -> Self {
        self.providers.push(Box::new(provider));
        self
    }
}

impl Default for CompositeCredential {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl CredentialProvider for CompositeCredential {
    async fn get_headers(&self) -> Result<HashMap<String, String>> {
        let mut all_headers = HashMap::new();
        for provider in &self.providers {
            let headers = provider.get_headers().await?;
            all_headers.extend(headers);
        }
        Ok(all_headers)
    }

    async fn refresh(&self) -> Result<()> {
        for provider in &self.providers {
            provider.refresh().await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_key_credential() {
        let cred = ApiKeyCredential::x_api_key("test-key");
        let headers = cred.get_headers().await.unwrap();
        assert_eq!(headers.get("X-API-Key"), Some(&"test-key".to_string()));
    }

    #[tokio::test]
    async fn test_bearer_token_credential() {
        let cred = BearerTokenCredential::new("my-token");
        let headers = cred.get_headers().await.unwrap();
        assert_eq!(
            headers.get("Authorization"),
            Some(&"Bearer my-token".to_string())
        );
    }

    #[tokio::test]
    async fn test_basic_auth_credential() {
        let cred = BasicAuthCredential::new("user", "pass");
        let headers = cred.get_headers().await.unwrap();
        let auth = headers.get("Authorization").unwrap();
        assert!(auth.starts_with("Basic "));
    }

    #[tokio::test]
    async fn test_composite_credential() {
        let cred = CompositeCredential::new()
            .add(ApiKeyCredential::x_api_key("key1"))
            .add(BearerTokenCredential::new("token1"));

        let headers = cred.get_headers().await.unwrap();
        assert!(headers.contains_key("X-API-Key"));
        assert!(headers.contains_key("Authorization"));
    }
}
