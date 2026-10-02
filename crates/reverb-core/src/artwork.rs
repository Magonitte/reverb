//! Capas em memória (F09, arquitetura §12). Falhas de um candidato tentam o próximo.

use std::io::Cursor;

use image::{codecs::jpeg::JpegEncoder, imageops::FilterType, ImageReader};

use crate::metadata::{
    provider::{http_client, HTTP_TIMEOUT},
    MetadataResult,
};
use crate::{CoreError, CoreResult};

pub const MAX_DOWNLOAD_BYTES: usize = 15 * 1024 * 1024;
pub const MAX_DIMENSION: u32 = 1200;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverCandidate {
    pub url: String,
    pub source: String,
}

#[derive(Debug, Clone)]
pub struct Artwork {
    pub jpeg: Vec<u8>,
    pub cover_source: String,
}

/// Usa somente candidatos já identificados; a miniatura do YouTube vem por último.
pub fn candidates(
    metadata: &MetadataResult,
    thumbnail: Option<&str>,
    confidence_auto_apply: f64,
) -> Vec<CoverCandidate> {
    let mut covers = Vec::new();
    let mut push = |url: &str, source: &str| {
        if !url.trim().is_empty() && !covers.iter().any(|c: &CoverCandidate| c.url == url) {
            covers.push(CoverCandidate {
                url: url.to_string(),
                source: source.to_string(),
            });
        }
    };
    if let Some(url) = metadata.fields.cover_url.as_deref() {
        let provider = metadata
            .candidates
            .iter()
            .find(|c| {
                c.score >= confidence_auto_apply && c.candidate.cover_url.as_deref() == Some(url)
            })
            .map(|c| c.candidate.provider.as_str());
        if matches!(
            metadata.source.as_str(),
            "deezer" | "itunes" | "musicbrainz" | "user"
        ) {
            push(url, &metadata.source);
        } else if metadata.source == "youtube_music" {
            if let Some(provider) = provider {
                push(url, provider);
            }
        }
    }
    for provider in ["deezer", "itunes", "musicbrainz"] {
        for c in &metadata.candidates {
            if c.candidate.provider == provider && c.score >= confidence_auto_apply {
                if let Some(url) = c.candidate.cover_url.as_deref() {
                    push(url, provider);
                }
            }
        }
    }
    if let Some(url) = thumbnail {
        push(url, "youtube");
    }
    covers
}

/// Decodifica com limite de memória, recorta no centro e codifica JPEG q90.
pub fn process(bytes: &[u8]) -> CoreResult<Vec<u8>> {
    if bytes.len() > MAX_DOWNLOAD_BYTES {
        return Err(CoreError::coded("artwork_size", "capa excede 15 MB"));
    }
    let mut reader = ImageReader::new(Cursor::new(bytes)).with_guessed_format()?;
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let mut decoded = reader.decode().map_err(image_error)?;
    let (width, height) = (decoded.width(), decoded.height());
    let side = width.min(height);
    if side == 0 {
        return Err(CoreError::coded("artwork_decode", "capa sem dimensões"));
    }
    if f64::from(width.max(height)) / f64::from(side) > 1.05 {
        decoded = decoded.crop_imm((width - side) / 2, (height - side) / 2, side, side);
    }
    if decoded.width().max(decoded.height()) > MAX_DIMENSION {
        decoded = decoded.resize(MAX_DIMENSION, MAX_DIMENSION, FilterType::Lanczos3);
    }
    let mut jpeg = Vec::new();
    JpegEncoder::new_with_quality(&mut jpeg, 90)
        .encode_image(&decoded.to_rgb8())
        .map_err(image_error)?;
    Ok(jpeg)
}

fn image_error(error: image::ImageError) -> CoreError {
    CoreError::coded("artwork_decode", error.to_string())
}

pub struct ArtworkClient {
    client: reqwest::Client,
}

impl Default for ArtworkClient {
    fn default() -> Self {
        Self::new(http_client())
    }
}

impl ArtworkClient {
    pub fn new(client: reqwest::Client) -> Self {
        Self { client }
    }

    /// Sem capa quando todos falham; não grava arquivos nem interrompe o job.
    pub async fn fetch(&self, candidates: &[CoverCandidate]) -> Option<Artwork> {
        for candidate in candidates {
            match self.download(&candidate.url).await {
                Ok(bytes) => {
                    let result = tokio::task::spawn_blocking(move || process(&bytes)).await;
                    match result {
                        Ok(Ok(jpeg)) => {
                            return Some(Artwork {
                                jpeg,
                                cover_source: candidate.source.clone(),
                            })
                        }
                        Ok(Err(error)) => {
                            tracing::warn!(source = %candidate.source, %error, "capa inválida")
                        }
                        Err(error) => {
                            tracing::warn!(source = %candidate.source, %error, "processamento da capa falhou")
                        }
                    }
                }
                Err(error) => {
                    tracing::warn!(source = %candidate.source, %error, "download da capa falhou")
                }
            }
        }
        None
    }

    async fn download(&self, url: &str) -> CoreResult<Vec<u8>> {
        let mut response = self
            .client
            .get(url)
            .timeout(HTTP_TIMEOUT)
            .send()
            .await?
            .error_for_status()?;
        if response
            .content_length()
            .is_some_and(|n| n > MAX_DOWNLOAD_BYTES as u64)
        {
            return Err(CoreError::coded("artwork_size", "capa excede 15 MB"));
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await? {
            if chunk.len() > MAX_DOWNLOAD_BYTES - bytes.len() {
                return Err(CoreError::coded("artwork_size", "capa excede 15 MB"));
            }
            bytes.extend_from_slice(&chunk);
        }
        Ok(bytes)
    }
}

#[cfg(test)]
mod tests;
