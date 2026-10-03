//! Modelo de nomes (§13). Só os separadores do modelo criam diretórios.

use std::path::{Path, PathBuf};

use crate::metadata::ContentType;
use crate::settings::{validate_file_template, Language};
use crate::tagging::TrackTags;
use crate::{CoreError, CoreResult};

use super::sanitize::{MAX_COMPONENT_CHARS, MAX_PATH_CHARS};
use super::{sanitize_path, unique_path};

#[derive(Debug, Clone)]
pub struct TemplateContext {
    pub output_dir: PathBuf,
    pub extension: String,
    pub language: Language,
    pub content_type: ContentType,
    pub auto_organize: bool,
    pub channel: Option<String>,
    pub source_id: Option<String>,
    pub playlist: Option<String>,
    pub playlist_index: Option<u32>,
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.filter(|s| !s.trim().is_empty())
}

fn variable(body: &str, tags: &TrackTags, ctx: &TemplateContext) -> String {
    let (name, width) = body.split_once(':').map_or((body, 0), |(name, width)| {
        (name, width.parse::<usize>().unwrap_or(0))
    });
    let artist = nonempty(tags.artist.as_deref()).unwrap_or(match ctx.language {
        Language::PtBr => "Artista desconhecido",
        Language::En => "Unknown artist",
    });
    let number = match name {
        "track" => tags.track_no,
        "disc" => tags.disc_no,
        "year" => tags.year,
        "playlist_index" => ctx.playlist_index,
        _ => None,
    };
    let value = match name {
        "track" | "disc" | "year" | "playlist_index" => {
            number.map(|n| format!("{n:0width$}")).unwrap_or_default()
        }
        "artist" => artist.to_owned(),
        "albumartist" => nonempty(tags.album_artist.as_deref())
            .unwrap_or(artist)
            .to_owned(),
        "album" => nonempty(tags.album.as_deref())
            .unwrap_or("Singles")
            .to_owned(),
        "title" => tags.title.clone(),
        "genre" => tags.genre.clone().unwrap_or_default(),
        "channel" => ctx.channel.clone().unwrap_or_default(),
        "source_id" => ctx.source_id.clone().unwrap_or_default(),
        "playlist" => ctx.playlist.clone().unwrap_or_default(),
        _ => String::new(), // O validador já rejeitou nomes desconhecidos.
    };
    if value.trim().is_empty() {
        String::new()
    } else {
        value.replace(['/', '\\'], "_")
    }
}

fn component(template: &str, tags: &TrackTags, ctx: &TemplateContext) -> String {
    let mut rest = template;
    let mut result = String::new();
    while let Some(open) = rest.find('{') {
        result.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let close = after.find('}').expect("modelo validado");
        let value = variable(&after[..close], tags, ctx);
        rest = &after[close + 1..];
        if value.is_empty() {
            if result.ends_with(" - ") {
                result.truncate(result.len() - 3);
            } else if let Some(after_separator) = rest.strip_prefix(" - ") {
                rest = after_separator;
            }
        } else {
            result.push_str(&value);
        }
    }
    result.push_str(rest);
    result
}

pub(crate) fn check_path(path: &Path) -> CoreResult<()> {
    if path.to_string_lossy().chars().count() > MAX_PATH_CHARS
        || path
            .file_name()
            .is_some_and(|s| s.to_string_lossy().chars().count() > MAX_COMPONENT_CHARS)
    {
        return Err(CoreError::invalid(
            "A pasta de destino não deixa espaço para um nome dentro dos limites",
        ));
    }
    Ok(())
}

/// Retorna erro se a raiz e os diretórios já excederem 240 caracteres, mesmo com título mínimo.
pub fn render(template: &str, tags: &TrackTags, ctx: &TemplateContext) -> CoreResult<PathBuf> {
    let path = unique_path(&render_destination(template, tags, ctx)?);
    check_path(&path)?;
    Ok(path)
}

/// Destination before collisions, for editing a file already at its correct path.
pub fn render_destination(
    template: &str,
    tags: &TrackTags,
    ctx: &TemplateContext,
) -> CoreResult<PathBuf> {
    let model = if ctx.content_type == ContentType::Other {
        match ctx.language {
            Language::PtBr => "Outros/{channel}/{title}",
            Language::En => "Other/{channel}/{title}",
        }
    } else if !ctx.auto_organize {
        "{artist} - {title}"
    } else {
        template
    };
    validate_file_template(model)?;
    let mut parts: Vec<String> = model
        .split(['/', '\\'])
        .map(|part| component(part, tags, ctx))
        .filter(|part| !part.trim().is_empty())
        .collect();
    if parts.is_empty() {
        parts.push("_".to_owned());
    }
    let refs: Vec<&str> = parts.iter().map(String::as_str).collect();
    let path = sanitize_path(&ctx.output_dir, &refs, &ctx.extension);
    check_path(&path)?;
    Ok(path)
}

pub fn preview(template: &str, settings: &crate::Settings) -> CoreResult<String> {
    validate_file_template(template)?;
    let tags = TrackTags {
        title: "Never Gonna Give You Up".into(),
        artist: Some("Rick Astley".into()),
        album_artist: Some("Rick Astley".into()),
        album: Some("Whenever You Need Somebody".into()),
        track_no: Some(1),
        disc_no: Some(1),
        year: Some(1987),
        genre: Some("Pop".into()),
        ..Default::default()
    };
    let root = crate::paths::resolve_output_dir(settings);
    let ctx = TemplateContext {
        output_dir: root.clone(),
        extension: "opus".into(),
        language: settings.language,
        content_type: ContentType::Music,
        auto_organize: settings.auto_organize,
        channel: Some("Rick Astley".into()),
        source_id: Some("lYBUbBu4W08".into()),
        playlist: Some("Favorites".into()),
        playlist_index: Some(1),
    };
    let path = render(template, &tags, &ctx)?;
    Ok(path
        .strip_prefix(&root)
        .unwrap_or(&path)
        .to_string_lossy()
        .into_owned())
}

#[cfg(test)]
mod tests;
