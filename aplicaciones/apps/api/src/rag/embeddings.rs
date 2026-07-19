use crate::config::Config;
use anyhow::{anyhow, bail, Context, Result};
use fastembed::{EmbeddingModel, TextEmbedding, TextInitOptions};
use sha2::{Digest, Sha256};
use std::sync::{Arc, Mutex};

const LOCAL_HASHING_DIMENSIONS: usize = 384;

pub struct EmbeddingService {
    provider: Arc<dyn EmbeddingProvider>,
    fallback: String,
}

impl EmbeddingService {
    pub fn from_config(config: &Config) -> Result<Self> {
        let provider: Arc<dyn EmbeddingProvider> =
            match config.embedding_provider.to_ascii_lowercase().as_str() {
                "fastembed" => Arc::new(FastEmbedProvider::new(&config.embedding_model)?),
                "local-hashing" | "local-hashing-v1" => {
                    Arc::new(LocalHashingProvider::new(&config.embedding_model))
                }
                other => bail!("unsupported EMBEDDING_PROVIDER: {other}"),
            };

        Ok(Self {
            provider,
            fallback: config.embedding_fallback.clone(),
        })
    }

    pub fn provider_name(&self) -> &str {
        self.provider.provider_name()
    }

    pub fn model_name(&self) -> &str {
        self.provider.model_name()
    }

    pub fn fallback(&self) -> &str {
        &self.fallback
    }

    pub fn dimensions(&self) -> usize {
        self.provider.dimensions()
    }

    pub async fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        let provider = self.provider.clone();
        tokio::task::spawn_blocking(move || provider.embed_documents(texts))
            .await
            .context("embedding task panicked")?
    }

    pub async fn embed_query(&self, text: String) -> Result<Vec<f32>> {
        let provider = self.provider.clone();
        tokio::task::spawn_blocking(move || provider.embed_query(text))
            .await
            .context("embedding task panicked")?
    }
}

trait EmbeddingProvider: Send + Sync {
    fn provider_name(&self) -> &str;
    fn model_name(&self) -> &str;
    fn dimensions(&self) -> usize;
    fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>>;
    fn embed_query(&self, text: String) -> Result<Vec<f32>>;
}

struct FastEmbedProvider {
    model_name: String,
    model: EmbeddingModel,
    dimensions: usize,
    uses_e5_prefix: bool,
    embedding: Mutex<Option<TextEmbedding>>,
}

impl FastEmbedProvider {
    fn new(model_name: &str) -> Result<Self> {
        let model = parse_fastembed_model(model_name)?;
        let info = TextEmbedding::get_model_info(&model)?;
        let dimensions = info.dim;
        Ok(Self {
            model_name: model_name.to_owned(),
            model,
            dimensions,
            uses_e5_prefix: model_name.to_ascii_lowercase().contains("e5"),
            embedding: Mutex::new(None),
        })
    }

    fn embed_prefixed(&self, texts: Vec<String>, is_query: bool) -> Result<Vec<Vec<f32>>> {
        if texts.is_empty() {
            return Ok(vec![]);
        }

        let inputs = if self.uses_e5_prefix {
            let prefix = if is_query { "query" } else { "passage" };
            texts
                .into_iter()
                .map(|text| format!("{prefix}: {text}"))
                .collect()
        } else {
            texts
        };

        let mut guard = self
            .embedding
            .lock()
            .map_err(|_| anyhow!("embedding provider lock poisoned"))?;
        if guard.is_none() {
            let options =
                TextInitOptions::new(self.model.clone()).with_show_download_progress(false);
            *guard = Some(TextEmbedding::try_new(options)?);
        }
        guard
            .as_mut()
            .expect("embedding initialized")
            .embed(inputs, None)
            .context("fastembed failed to embed text")
    }
}

impl EmbeddingProvider for FastEmbedProvider {
    fn provider_name(&self) -> &str {
        "fastembed"
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn dimensions(&self) -> usize {
        self.dimensions
    }

    fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        self.embed_prefixed(texts, false)
    }

    fn embed_query(&self, text: String) -> Result<Vec<f32>> {
        self.embed_prefixed(vec![text], true)?
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("fastembed returned no query embedding"))
    }
}

struct LocalHashingProvider {
    model_name: String,
}

impl LocalHashingProvider {
    fn new(model_name: &str) -> Self {
        Self {
            model_name: model_name.to_owned(),
        }
    }

    fn embed_text(text: &str) -> Vec<f32> {
        let mut vector = vec![0.0; LOCAL_HASHING_DIMENSIONS];
        for token in text
            .split_whitespace()
            .map(normalize)
            .filter(|token| !token.is_empty())
        {
            let digest = Sha256::digest(token.as_bytes());
            let idx =
                u16::from_le_bytes([digest[0], digest[1]]) as usize % LOCAL_HASHING_DIMENSIONS;
            vector[idx] += 1.0;
        }
        normalize_vector(vector)
    }
}

impl EmbeddingProvider for LocalHashingProvider {
    fn provider_name(&self) -> &str {
        "local-hashing"
    }

    fn model_name(&self) -> &str {
        &self.model_name
    }

    fn dimensions(&self) -> usize {
        LOCAL_HASHING_DIMENSIONS
    }

    fn embed_documents(&self, texts: Vec<String>) -> Result<Vec<Vec<f32>>> {
        Ok(texts.iter().map(|text| Self::embed_text(text)).collect())
    }

    fn embed_query(&self, text: String) -> Result<Vec<f32>> {
        Ok(Self::embed_text(&text))
    }
}

fn parse_fastembed_model(model: &str) -> Result<EmbeddingModel> {
    match model.trim().to_ascii_lowercase().as_str() {
        "intfloat/multilingual-e5-small" | "multilinguale5small" => {
            Ok(EmbeddingModel::MultilingualE5Small)
        }
        "intfloat/multilingual-e5-base" | "multilinguale5base" => {
            Ok(EmbeddingModel::MultilingualE5Base)
        }
        "intfloat/multilingual-e5-large" | "multilinguale5large" => {
            Ok(EmbeddingModel::MultilingualE5Large)
        }
        value => value
            .parse()
            .map_err(|err| anyhow!("unsupported fastembed model {model}: {err}")),
    }
}

fn normalize(token: &str) -> String {
    token
        .chars()
        .filter(|c| c.is_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn normalize_vector(mut vector: Vec<f32>) -> Vec<f32> {
    let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_hashing_embedding_has_stable_size() {
        let provider = LocalHashingProvider::new("local-hashing-v1");
        assert_eq!(
            provider
                .embed_query("cita previa".to_owned())
                .unwrap()
                .len(),
            384
        );
        assert_eq!(
            provider.embed_query("cita previa".to_owned()).unwrap(),
            provider.embed_query("cita previa".to_owned()).unwrap()
        );
    }

    #[test]
    fn maps_hugging_face_model_name_to_fastembed_model() {
        assert_eq!(
            parse_fastembed_model("intfloat/multilingual-e5-small").unwrap(),
            EmbeddingModel::MultilingualE5Small
        );
    }
}
