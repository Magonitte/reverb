//! `MetadataService`: o pipeline de metadados do §11. `resolve_source` roda **antes** do download
//! (analisa o vídeo, classifica e decide a fonte final) e `identify` roda **depois** (com a duração
//! real do arquivo). `preview` simula os dois sem baixar nada.

use std::sync::Arc;

use futures_util::future::join_all;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::cache::{Cache, CachedProvider, Clock};
use super::content_type::{content_type, is_music_host, ContentType};
use super::deezer::{self, Deezer};
use super::itunes::Itunes;
use super::model::{
    Bucket, MetadataFields, MetadataOverride, MetadataResult, ScoredCandidate, SOURCE_USER,
    SOURCE_YOUTUBE, SOURCE_YOUTUBE_MUSIC,
};
use super::musicbrainz::MusicBrainz;
use super::normalize::norm_plain;
use super::official::{find_official, OfficialMatch};
use super::parse_title::{clean_channel, parse_title, split_artists};
use super::provider::{http_client, Candidate, Endpoints, MetadataProvider, Query};
use super::score::{score, DurationTolerance, Subject};
use crate::backend::DownloadBackend;
use crate::db::Db;
use crate::settings::{Settings, SettingsService};
use crate::ytdlp::errors::{DownloadError, ErrorKind};
use crate::ytdlp::{Analysis, VideoInfo};

/// Quantos candidatos ficam no resultado (e em `review_candidates_json`).
const MAX_CANDIDATES: usize = 5;

/// Decisão do `resolve_source`: de onde baixar e o que se sabe do vídeo.
#[derive(Debug, Clone)]
pub struct SourcePlan {
    /// URL que o download deve usar (a oficial, se a fonte foi trocada).
    pub url: String,
    /// Análise do vídeo que será baixado; `None` se a análise falhou (o job segue mesmo assim).
    pub video: Option<VideoInfo>,
    /// URL original, quando a fonte foi trocada pela versão oficial.
    pub switched_from: Option<String>,
    pub official: Option<OfficialMatch>,
    pub content_type: ContentType,
}

impl SourcePlan {
    pub fn switched(&self) -> bool {
        self.switched_from.is_some()
    }

    /// Plano de quem não analisa nada (sem serviço, ou análise que falhou).
    pub fn unknown(url: &str) -> Self {
        Self {
            url: url.to_string(),
            video: None,
            switched_from: None,
            official: None,
            content_type: if is_music_host(url) {
                ContentType::Music
            } else {
                ContentType::Other
            },
        }
    }
}

/// O que `identify` precisa além do plano.
pub struct IdentifyInput<'a> {
    pub plan: &'a SourcePlan,
    /// Título que o yt-dlp informou ao baixar (serve quando a análise falhou).
    pub fallback_title: &'a str,
    /// Duração real do arquivo (ffprobe) ou, na falta, a do YouTube.
    pub duration_s: Option<f64>,
    /// Edição do usuário: vence tudo e pula os provedores.
    pub user_override: Option<&'a Value>,
    /// Sobrescreve `fetchMetadata` só para este job.
    pub fetch_metadata: Option<bool>,
}

pub struct MetadataService {
    pub keyed: crate::quality::providers::KeyedProviders,
    backend: Arc<dyn DownloadBackend>,
    settings: Arc<SettingsService>,
    providers: Vec<Arc<dyn MetadataProvider>>,
}

/// Base de partida (§11 passo 2): oficial do YouTube Music ou análise do título.
struct Base {
    fields: MetadataFields,
    official: bool,
    subject: Subject,
}

impl MetadataService {
    /// Provedores reais (Deezer → iTunes → MusicBrainz) com cache de 30 dias na frente.
    /// Os limitadores de taxa vivem em cada provedor: **uma** instância do serviço por processo.
    pub fn new(
        backend: Arc<dyn DownloadBackend>,
        settings: Arc<SettingsService>,
        db: Db,
        endpoints: &Endpoints,
        clock: Clock,
    ) -> Self {
        let client = http_client();
        let cache = Cache::new(db, clock);
        let raw: Vec<Arc<dyn MetadataProvider>> = vec![
            Arc::new(Deezer::new(
                client.clone(),
                &endpoints.deezer,
                Deezer::limiter(),
            )),
            Arc::new(Itunes::new(
                client.clone(),
                &endpoints.itunes,
                Itunes::limiter(),
            )),
            Arc::new(MusicBrainz::new(
                client,
                &endpoints.musicbrainz,
                &endpoints.cover_art,
                MusicBrainz::limiter(),
            )),
        ];
        let providers = raw
            .into_iter()
            .map(|inner| {
                Arc::new(CachedProvider::new(inner, cache.clone())) as Arc<dyn MetadataProvider>
            })
            .collect();
        Self::with_providers(backend, settings, providers)
    }

    pub fn with_providers(
        backend: Arc<dyn DownloadBackend>,
        settings: Arc<SettingsService>,
        providers: Vec<Arc<dyn MetadataProvider>>,
    ) -> Self {
        Self {
            keyed: crate::quality::providers::KeyedProviders::new(settings.clone()),
            backend,
            settings,
            providers,
        }
    }

    // -----------------------------------------------------------------------------------------
    // resolve_source (antes do download)
    // -----------------------------------------------------------------------------------------

    /// Analisa o vídeo e decide a fonte final. Só o cancelamento vira erro: se a análise falhar, o
    /// job segue com a URL original (o download dirá o que houver de errado).
    pub async fn resolve_source(
        &self,
        url: &str,
        has_user_override: bool,
        cancel: &CancellationToken,
    ) -> Result<SourcePlan, DownloadError> {
        let analysis = match self.backend.analyze(url, cancel).await {
            Ok(analysis) => analysis,
            Err(error) if error.kind == ErrorKind::Cancelled => return Err(error),
            Err(error) => {
                tracing::warn!(
                    kind = error.kind.as_str(),
                    "análise do vídeo falhou; seguindo sem ela"
                );
                return Ok(SourcePlan::unknown(url));
            }
        };
        let Analysis::Video { info } = analysis else {
            return Ok(SourcePlan::unknown(url));
        };
        // Preserve the timeline of full albums before dividing their chapters.
        let chapter_album = info.chapters.len() >= 2 && info.duration.is_some_and(|d| d > 600.0);
        let prefer =
            self.settings.get().prefer_official_audio && !has_user_override && !chapter_album;
        self.plan_source(url, *info, prefer, prefer, cancel).await
    }

    /// `look_up`: procura a versão oficial (e a reporta); `switch`: troca a fonte por ela.
    async fn plan_source(
        &self,
        url: &str,
        video: VideoInfo,
        look_up: bool,
        switch: bool,
        cancel: &CancellationToken,
    ) -> Result<SourcePlan, DownloadError> {
        let settings = self.settings.get();
        let content = content_type(&video, Some(url));
        let mut official = None;
        let mut chosen = None;
        if look_up
            && content == ContentType::Music
            && !video.is_official_track
            && !settings.offline_mode
        {
            if let Some(found) = self.search_official(&video, cancel).await {
                official = Some(found.matched.clone());
                if switch {
                    chosen = Some(found);
                }
            }
            if cancel.is_cancelled() {
                return Err(DownloadError::cancelled());
            }
        }
        Ok(match chosen {
            Some(found) => {
                let content = content_type(&found.info, Some(&found.matched.url));
                SourcePlan {
                    url: found.matched.url.clone(),
                    video: Some(found.info),
                    switched_from: Some(url.to_string()),
                    official,
                    content_type: content,
                }
            }
            None => SourcePlan {
                url: url.to_string(),
                video: Some(video),
                switched_from: None,
                official,
                content_type: content,
            },
        })
    }

    /// Texto primeiro (com os filtros do E1); se nada servir, o ISRC inferido pelo Deezer.
    ///
    /// O ISRC inferido é só um plano B: o 1º resultado do Deezer para FX3 é a gravação da
    /// coletânea (`GBARL0600786`), cujo ISRC levaria a outra faixa que não a oficial do álbum
    /// original. Um ISRC **informado** pelo chamador (importação) tem prioridade em
    /// `find_official_version`.
    async fn search_official(
        &self,
        video: &VideoInfo,
        cancel: &CancellationToken,
    ) -> Option<super::official::OfficialFound> {
        if let Some(found) = find_official(self.backend.as_ref(), video, None, cancel).await {
            return Some(found);
        }
        let isrc = self.infer_isrc(video).await?;
        find_official(self.backend.as_ref(), video, Some(&isrc), cancel).await
    }

    /// Busca rápida no Deezer pelo artista + título analisados: o ISRC do melhor candidato com
    /// pontuação ≥ `confidenceAutoApply`.
    async fn infer_isrc(&self, video: &VideoInfo) -> Option<String> {
        let settings = self.settings.get();
        let provider = self.providers.iter().find(|p| p.id() == deezer::ID)?;
        let subject = super::official::subject_of(video);
        let query = Query::new(
            subject.title.clone(),
            subject.artists.clone(),
            subject.duration_s,
        );
        let mut best: Option<(f64, Candidate)> = None;
        for candidate in provider.search(&query).await {
            let value = score(
                &subject,
                &candidate_subject(&candidate),
                DurationTolerance::Clip,
            );
            if value >= settings.confidence_auto_apply
                && best.as_ref().is_none_or(|(top, _)| value > *top)
            {
                best = Some((value, candidate));
            }
        }
        let (_, candidate) = best?;
        match candidate.isrc.clone() {
            Some(isrc) => Some(isrc),
            None => provider.details(candidate).await.isrc,
        }
    }

    /// `find_official_version`: com ISRC informado, ele vem primeiro (E1).
    pub async fn find_official_version(
        &self,
        video: &VideoInfo,
        isrc: Option<&str>,
        cancel: &CancellationToken,
    ) -> Option<OfficialMatch> {
        if self.settings.get().offline_mode {
            return None;
        }
        match isrc {
            Some(isrc) => {
                let found = find_official(self.backend.as_ref(), video, Some(isrc), cancel).await;
                match found {
                    Some(found) => Some(found.matched),
                    None => self.search_official(video, cancel).await.map(|f| f.matched),
                }
            }
            None => self.search_official(video, cancel).await.map(|f| f.matched),
        }
    }

    // -----------------------------------------------------------------------------------------
    // identify (depois do download)
    // -----------------------------------------------------------------------------------------

    pub async fn identify(&self, input: IdentifyInput<'_>) -> MetadataResult {
        let settings = self.settings.get();
        let plan = input.plan;
        let base = self.base(plan, input.fallback_title, input.duration_s, &settings);
        let mut result = MetadataResult {
            confidence: if base.official { 1.0 } else { 0.0 },
            source: if base.official {
                SOURCE_YOUTUBE_MUSIC
            } else {
                SOURCE_YOUTUBE
            }
            .to_string(),
            bucket: if base.official {
                Bucket::Auto
            } else {
                Bucket::None
            },
            candidates: Vec::new(),
            content_type: plan.content_type,
            isrc: None,
            official: plan.official.clone(),
            fields: base.fields.clone(),
        };

        // A edição do usuário vence tudo e pula os provedores.
        if let Some(value) = input.user_override.filter(|v| !v.is_null()) {
            MetadataOverride::from_value(value).apply(&mut result.fields);
            result.confidence = 1.0;
            result.source = SOURCE_USER.to_string();
            result.bucket = Bucket::Auto;
            return result;
        }

        let fetch = input.fetch_metadata.unwrap_or(settings.fetch_metadata);
        if plan.content_type != ContentType::Music || !fetch || settings.offline_mode {
            return result;
        }
        self.enrich(base, &settings, &mut result).await;
        result
    }

    fn base(
        &self,
        plan: &SourcePlan,
        fallback_title: &str,
        duration_s: Option<f64>,
        settings: &Settings,
    ) -> Base {
        let music = plan.content_type == ContentType::Music;
        let mut fields = MetadataFields::default();
        let mut official = false;
        let duration = duration_s.or_else(|| plan.video.as_ref().and_then(|v| v.duration));
        match plan.video.as_ref() {
            Some(video) if video.is_official_track => {
                official = true;
                fields.title = video.track.clone().unwrap_or_else(|| video.title.clone());
                let artists = if video.artists.is_empty() {
                    video
                        .artist
                        .as_deref()
                        .map(split_artists)
                        .unwrap_or_default()
                } else {
                    video.artists.clone()
                };
                fields.artist = video.artist.clone();
                fields.set_artists(artists);
                fields.album = video.album.clone();
                fields.year = video.release_year;
            }
            Some(video) => {
                let channel = video.channel.as_deref().or(video.uploader.as_deref());
                if music && settings.extract_title_from_video {
                    let parsed = parse_title(&video.title, channel);
                    fields.title = parsed.title;
                    fields.artist = parsed.artist;
                    fields.artists = parsed.artists;
                } else if music {
                    fields.title = video.title.clone();
                    fields.artist = channel.and_then(clean_channel);
                    fields.artists = fields.artist.iter().cloned().collect();
                } else {
                    // Conteúdo que não é música: título do vídeo e canal como artista.
                    fields.title = video.title.clone();
                    fields.artist = channel.map(str::to_string);
                    fields.artists = fields.artist.iter().cloned().collect();
                }
            }
            None => {
                if music && settings.extract_title_from_video {
                    let parsed = parse_title(fallback_title, None);
                    fields.title = parsed.title;
                    fields.artist = parsed.artist;
                    fields.artists = parsed.artists;
                } else {
                    fields.title = fallback_title.to_string();
                }
            }
        }
        let subject = Subject {
            title: fields.title.clone(),
            artists: fields.artists.clone(),
            duration_s: duration,
        };
        Base {
            fields,
            official,
            subject,
        }
    }

    /// Passos 4 e 5 do §11: consulta os provedores em paralelo, pontua e decide.
    async fn enrich(&self, base: Base, settings: &Settings, result: &mut MetadataResult) {
        let query = Query::new(
            base.subject.title.clone(),
            base.subject.artists.clone(),
            base.subject.duration_s,
        );
        let tolerance = if base.official {
            DurationTolerance::Normal
        } else {
            DurationTolerance::Clip
        };
        let mut lists = join_all(self.providers.iter().map(|p| p.search(&query))).await;
        if settings.secrets_status().spotify {
            match self.keyed.spotify_search(&query).await {
                Ok(candidates) => lists.push(candidates),
                Err(error) => tracing::warn!(kind = error.kind(), "Spotify metadata unavailable"),
            }
        }
        if settings.secrets_status().discogs {
            match self.keyed.discogs_search(&query).await {
                Ok(candidates) => lists.push(candidates),
                Err(error) => tracing::warn!(kind = error.kind(), "Discogs metadata unavailable"),
            }
        }
        let mut ranked: Vec<(f64, Candidate)> = lists
            .into_iter()
            .flatten()
            .map(|candidate| {
                let value = score(&base.subject, &candidate_subject(&candidate), tolerance);
                (value, candidate)
            })
            .collect();
        // Empate de pontuação: vence o título idêntico ao buscado (sem "(2022 Remaster)" etc.) e,
        // depois, a duração mais próxima (a gravação certa, não a coletânea).
        let wanted = base.subject.duration_s;
        let wanted_title = norm_plain(&base.subject.title);
        let key = |(value, candidate): &(f64, Candidate)| {
            let delta = match (wanted, candidate.duration_s) {
                (Some(a), Some(b)) => ((a - b).abs() * 1000.0) as i64,
                _ => i64::MAX,
            };
            (
                std::cmp::Reverse((value * 10_000.0).round() as i64),
                norm_plain(&candidate.title) != wanted_title,
                delta,
            )
        };
        ranked.sort_by_key(key);
        ranked.truncate(MAX_CANDIDATES);
        let Some((best_score, best)) = ranked.first().cloned() else {
            return;
        };
        let auto = best_score >= settings.confidence_auto_apply;

        // F09 reutiliza estas capas, inclusive quando a base já é oficial.
        result.candidates = ranked
            .iter()
            .map(|(value, candidate)| ScoredCandidate {
                score: *value,
                candidate: candidate.clone(),
            })
            .collect();
        if base.official {
            // Base oficial: o candidato só completa o que falta, e só se for confiável.
            if auto {
                let detailed = self.details(best).await;
                result.fields.fill_missing(&detailed);
                result.isrc = detailed.isrc.clone();
                result.candidates[0].candidate = detailed;
            }
            return;
        }

        if auto {
            let detailed = self.details(best).await;
            result.fields.apply_candidate(&detailed);
            result.isrc = detailed.isrc.clone();
            result.confidence = best_score;
            result.source = detailed.provider.clone();
            result.bucket = Bucket::Auto;
            result.candidates[0].candidate = detailed;
        } else if best_score >= settings.confidence_review {
            result.confidence = best_score;
            result.bucket = Bucket::Review;
        } else {
            result.candidates.clear();
        }
    }

    async fn details(&self, candidate: Candidate) -> Candidate {
        match self.providers.iter().find(|p| p.id() == candidate.provider) {
            Some(provider) => provider.details(candidate).await,
            None => candidate,
        }
    }

    // -----------------------------------------------------------------------------------------
    // preview e busca manual
    // -----------------------------------------------------------------------------------------

    /// Simulação sem baixar (usada no Preview): o que `resolve_source` + `identify` fariam.
    /// `use_official`: `None` segue `preferOfficialAudio`.
    pub async fn preview(
        &self,
        url: &str,
        video: Option<VideoInfo>,
        use_official: Option<bool>,
        cancel: &CancellationToken,
    ) -> Result<MetadataResult, DownloadError> {
        let video =
            match video {
                Some(video) => video,
                None => match self.backend.analyze(url, cancel).await? {
                    Analysis::Video { info } => *info,
                    Analysis::Collection { .. } => return Err(DownloadError::new(
                        ErrorKind::Unavailable,
                        "a pré-visualização de metadados vale para um vídeo, não para uma coleção",
                    )),
                },
            };
        let switch = use_official.unwrap_or_else(|| self.settings.get().prefer_official_audio);
        let plan = self.plan_source(url, video, true, switch, cancel).await?;
        let duration = plan.video.as_ref().and_then(|v| v.duration);
        let title = plan
            .video
            .as_ref()
            .map(|v| v.title.clone())
            .unwrap_or_default();
        Ok(self
            .identify(IdentifyInput {
                plan: &plan,
                fallback_title: &title,
                duration_s: duration,
                user_override: None,
                fetch_metadata: None,
            })
            .await)
    }

    /// Busca manual (paridade com `search_metadata`): todos os provedores, melhor primeiro.
    pub async fn search(&self, text: &str) -> Vec<Candidate> {
        if self.settings.get().offline_mode || text.trim().is_empty() {
            return Vec::new();
        }
        let parsed = parse_title(text, None);
        let query = Query::new(parsed.title, parsed.artists, None);
        let subject = Subject {
            title: query.title.clone(),
            artists: query.artists.clone(),
            duration_s: None,
        };
        let lists = join_all(self.providers.iter().map(|p| p.search(&query))).await;
        let mut found: Vec<(f64, Candidate)> = lists
            .into_iter()
            .flatten()
            .map(|c| {
                (
                    score(&subject, &candidate_subject(&c), DurationTolerance::Normal),
                    c,
                )
            })
            .collect();
        found.sort_by_key(|(value, _)| std::cmp::Reverse((value * 10_000.0).round() as i64));
        found.into_iter().map(|(_, c)| c).collect()
    }
}

fn candidate_subject(candidate: &Candidate) -> Subject {
    Subject {
        title: candidate.title.clone(),
        artists: candidate.artists.clone(),
        duration_s: candidate.duration_s,
    }
}

#[cfg(test)]
mod tests;
