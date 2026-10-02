//! Tipos do resultado da identificação (arquitetura §11 e §15).

use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::content_type::ContentType;
use super::official::OfficialMatch;
use super::parse_title::split_artists;
use super::provider::Candidate;

/// Campos finais de uma faixa (o que o F09 grava nas tags).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MetadataFields {
    pub title: String,
    /// Texto de exibição (`A`, `A feat. B`, `A, B`).
    pub artist: Option<String>,
    pub artists: Vec<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub track_no: Option<u32>,
    pub track_total: Option<u32>,
    pub disc_no: Option<u32>,
    pub cover_url: Option<String>,
    pub mb_recording_id: Option<String>,
}

impl MetadataFields {
    /// Define os artistas a partir de uma lista (e o texto de exibição, se faltar).
    pub fn set_artists(&mut self, artists: Vec<String>) {
        if self.artist.is_none() && !artists.is_empty() {
            self.artist = Some(artists.join(", "));
        }
        self.artists = artists;
    }

    /// Copia os campos do candidato por cima (o que o candidato não tem, fica como está).
    pub fn apply_candidate(&mut self, candidate: &Candidate) {
        self.title = candidate.title.clone();
        if !candidate.artists.is_empty() {
            self.artists = candidate.artists.clone();
            self.artist = Some(candidate.artists.join(", "));
        }
        self.album = candidate.album.clone().or(self.album.take());
        self.album_artist = candidate.album_artist.clone().or(self.album_artist.take());
        self.year = candidate.year.or(self.year);
        self.genre = candidate.genre.clone().or(self.genre.take());
        self.track_no = candidate.track_no.or(self.track_no);
        self.track_total = candidate.track_total.or(self.track_total);
        self.disc_no = candidate.disc_no.or(self.disc_no);
        self.cover_url = candidate.cover_url.clone().or(self.cover_url.take());
        self.mb_recording_id = candidate
            .mb_recording_id
            .clone()
            .or(self.mb_recording_id.take());
    }

    /// Só preenche o que está vazio (base oficial completada por um candidato).
    pub fn fill_missing(&mut self, candidate: &Candidate) {
        if self.album.is_none() {
            self.album = candidate.album.clone();
        }
        if self.album_artist.is_none() {
            self.album_artist = candidate.album_artist.clone();
        }
        self.year = self.year.or(candidate.year);
        if self.genre.is_none() {
            self.genre = candidate.genre.clone();
        }
        self.track_no = self.track_no.or(candidate.track_no);
        self.track_total = self.track_total.or(candidate.track_total);
        self.disc_no = self.disc_no.or(candidate.disc_no);
        if self.cover_url.is_none() {
            self.cover_url = candidate.cover_url.clone();
        }
        if self.mb_recording_id.is_none() {
            self.mb_recording_id = candidate.mb_recording_id.clone();
        }
    }
}

/// Edição do usuário (vem do Preview): todos os campos são opcionais e têm prioridade máxima.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct MetadataOverride {
    pub title: Option<String>,
    pub artist: Option<String>,
    pub artists: Option<Vec<String>>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<u32>,
    pub genre: Option<String>,
    pub track_no: Option<u32>,
    pub track_total: Option<u32>,
    pub disc_no: Option<u32>,
    pub cover_url: Option<String>,
}

fn clean(text: Option<String>) -> Option<String> {
    text.map(|t| t.trim().to_string()).filter(|t| !t.is_empty())
}

impl MetadataOverride {
    /// Valor JSON inválido (ou `null`) vira "sem edição".
    pub fn from_value(value: &serde_json::Value) -> Self {
        serde_json::from_value(value.clone()).unwrap_or_default()
    }

    /// Sobrepõe os campos informados (texto vazio = não informado).
    pub fn apply(self, fields: &mut MetadataFields) {
        if let Some(title) = clean(self.title) {
            fields.title = title;
        }
        let artists = self
            .artists
            .map(|list| {
                list.into_iter()
                    .filter_map(|a| clean(Some(a)))
                    .collect::<Vec<_>>()
            })
            .filter(|list| !list.is_empty());
        match (clean(self.artist), artists) {
            (Some(display), Some(list)) => {
                fields.artist = Some(display);
                fields.artists = list;
            }
            (Some(display), None) => {
                fields.artists = split_artists(&display);
                fields.artist = Some(display);
            }
            (None, Some(list)) => {
                fields.artist = Some(list.join(", "));
                fields.artists = list;
            }
            (None, None) => {}
        }
        if let Some(album) = clean(self.album) {
            fields.album = Some(album);
        }
        if let Some(album_artist) = clean(self.album_artist) {
            fields.album_artist = Some(album_artist);
        }
        fields.year = self.year.or(fields.year);
        if let Some(genre) = clean(self.genre) {
            fields.genre = Some(genre);
        }
        fields.track_no = self.track_no.or(fields.track_no);
        fields.track_total = self.track_total.or(fields.track_total);
        fields.disc_no = self.disc_no.or(fields.disc_no);
        if let Some(cover) = clean(self.cover_url) {
            fields.cover_url = Some(cover);
        }
    }
}

/// Pedido de `metadata_preview`: a URL (e, se a UI já tiver, a análise do vídeo, que poupa uma
/// chamada ao yt-dlp).
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export, optional_fields)]
pub struct PreviewRequest {
    pub url: String,
    pub video: Option<crate::ytdlp::VideoInfo>,
    /// Simula com a versão oficial (`true`), sem ela (`false`) ou como as configurações (`None`).
    pub use_official: Option<bool>,
}

/// O que fazer com o resultado: aplicar sozinho, mandar para revisão ou nada.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum Bucket {
    Auto,
    Review,
    None,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct ScoredCandidate {
    pub score: f64,
    pub candidate: Candidate,
}

/// Resultado da identificação (gravado em `jobs.metadata_result_json`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct MetadataResult {
    pub fields: MetadataFields,
    /// 0–1. `1.0` para faixa oficial do YouTube Music e para edição do usuário.
    pub confidence: f64,
    /// `youtube_music`, `youtube`, `deezer`, `itunes`, `musicbrainz` ou `user`.
    pub source: String,
    pub bucket: Bucket,
    /// Até 5, do melhor para o pior (só quando há o que revisar).
    pub candidates: Vec<ScoredCandidate>,
    pub content_type: ContentType,
    pub isrc: Option<String>,
    /// Versão oficial encontrada para o vídeo (trocada ou apenas sugerida).
    pub official: Option<OfficialMatch>,
}

impl MetadataResult {
    pub fn needs_review(&self) -> bool {
        self.bucket == Bucket::Review
    }
}

pub const SOURCE_YOUTUBE_MUSIC: &str = "youtube_music";
pub const SOURCE_YOUTUBE: &str = "youtube";
pub const SOURCE_USER: &str = "user";

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> MetadataFields {
        MetadataFields {
            title: "Base".into(),
            artist: Some("Artista".into()),
            artists: vec!["Artista".into()],
            album: Some("Álbum".into()),
            year: Some(2000),
            ..MetadataFields::default()
        }
    }

    #[test]
    fn override_so_troca_o_que_foi_informado() {
        let mut fields = base();
        let edit = MetadataOverride::from_value(&serde_json::json!({
            "title": "  Novo  ", "genre": "Pop", "trackNo": 3, "album": "   "
        }));
        edit.apply(&mut fields);
        assert_eq!(fields.title, "Novo");
        assert_eq!(fields.genre.as_deref(), Some("Pop"));
        assert_eq!(fields.track_no, Some(3));
        assert_eq!(fields.album.as_deref(), Some("Álbum"));
        assert_eq!(fields.artist.as_deref(), Some("Artista"));
        assert_eq!(fields.year, Some(2000));
    }

    #[test]
    fn override_de_artista_separa_os_convidados() {
        let mut fields = base();
        MetadataOverride::from_value(&serde_json::json!({ "artist": "A feat. B" }))
            .apply(&mut fields);
        assert_eq!(fields.artist.as_deref(), Some("A feat. B"));
        assert_eq!(fields.artists, vec!["A", "B"]);
        MetadataOverride::from_value(&serde_json::json!({ "artists": ["X", "Y"] }))
            .apply(&mut fields);
        assert_eq!(fields.artist.as_deref(), Some("X, Y"));
        assert_eq!(fields.artists, vec!["X", "Y"]);
    }

    #[test]
    fn override_invalido_nao_muda_nada() {
        let mut fields = base();
        MetadataOverride::from_value(&serde_json::json!("lixo")).apply(&mut fields);
        MetadataOverride::from_value(&serde_json::Value::Null).apply(&mut fields);
        assert_eq!(fields, base());
    }

    #[test]
    fn candidato_completa_so_o_que_falta() {
        let mut fields = base();
        let mut candidate = Candidate::new("deezer", "1", "Outro título");
        candidate.album = Some("Outro álbum".into());
        candidate.genre = Some("Rock".into());
        candidate.track_no = Some(7);
        fields.fill_missing(&candidate);
        assert_eq!(fields.title, "Base");
        assert_eq!(fields.album.as_deref(), Some("Álbum"));
        assert_eq!(fields.genre.as_deref(), Some("Rock"));
        assert_eq!(fields.track_no, Some(7));
        fields.apply_candidate(&candidate);
        assert_eq!(fields.title, "Outro título");
        assert_eq!(fields.album.as_deref(), Some("Outro álbum"));
        assert_eq!(fields.year, Some(2000));
    }
}
