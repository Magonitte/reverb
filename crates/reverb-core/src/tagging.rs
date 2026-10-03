//! Tags portáveis (§12). Publica somente depois de reler e verificar a cópia de trabalho.

use std::io::Cursor;
use std::path::Path;

use lofty::file::{FileType, TaggedFile, TaggedFileExt};
use lofty::picture::{Picture, PictureType};
use lofty::prelude::{Accessor, ItemKey, TagExt};
use lofty::probe::Probe;
use lofty::tag::{Tag, TagType};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::loudness::ReplayGain;
use crate::lyrics::Lyrics;
use crate::metadata::MetadataFields;
use crate::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export)]
pub struct TagCover {
    pub mime_type: String,
    pub data: Vec<u8>,
}

/// Representação comum aos quatro formatos e ao editor da F10. `None` remove o campo.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase", default)]
#[ts(export)]
pub struct TrackTags {
    pub title: String,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub album_artist: Option<String>,
    pub year: Option<u32>,
    pub track_no: Option<u32>,
    pub track_total: Option<u32>,
    pub disc_no: Option<u32>,
    pub genre: Option<String>,
    pub lyrics: Option<String>,
    pub comment: Option<String>,
    pub isrc: Option<String>,
    pub cover: Option<TagCover>,
    pub replay_gain_track_gain: Option<String>,
    pub replay_gain_track_peak: Option<String>,
    /// Somente Opus: ganho em unidades de 1/256 dB relativo a -23 LUFS.
    pub r128_track_gain: Option<i16>,
}

impl TrackTags {
    pub fn from_metadata(fields: &MetadataFields, source_url: &str) -> Self {
        Self {
            title: fields.title.clone(),
            artist: fields.artist.clone(),
            album: fields.album.clone(),
            album_artist: fields.album_artist.clone(),
            year: fields.year,
            track_no: fields.track_no,
            track_total: fields.track_total,
            disc_no: fields.disc_no,
            genre: fields.genre.clone(),
            comment: Some(format!("Reverb · {source_url}")),
            ..Self::default()
        }
    }

    pub fn set_lyrics(&mut self, lyrics: &Lyrics) {
        self.lyrics = lyrics.synced.clone().or_else(|| lyrics.plain.clone());
    }

    pub fn set_replay_gain(&mut self, gain: &ReplayGain, opus: bool) {
        self.replay_gain_track_gain = Some(gain.track_gain_tag());
        self.replay_gain_track_peak = Some(gain.track_peak_tag());
        self.r128_track_gain = opus.then_some(gain.r128_track_gain);
    }
}

fn tag_error(error: impl std::fmt::Display) -> CoreError {
    CoreError::coded("tags", error.to_string())
}

fn open(path: &Path) -> CoreResult<TaggedFile> {
    let file = Probe::open(path)
        .map_err(tag_error)?
        .guess_file_type()
        .map_err(tag_error)?
        .read()
        .map_err(tag_error)?;
    match file.file_type() {
        FileType::Mpeg
        | FileType::Mp4
        | FileType::Opus
        | FileType::Vorbis
        | FileType::Flac
        | FileType::Wav => Ok(file),
        _ => Err(CoreError::coded("tags", "Formato de áudio não suportado")),
    }
}

fn text(tag: &Tag, key: ItemKey) -> Option<String> {
    tag.get_string(key).map(str::to_owned)
}

pub fn read_tags(path: &Path) -> CoreResult<TrackTags> {
    let file = open(path)?;
    let Some(tag) = file.primary_tag() else {
        return Ok(TrackTags::default());
    };
    let cover = tag
        .get_picture_type(PictureType::CoverFront)
        // MP4 covr não distingue o tipo de imagem.
        .or_else(|| {
            (file.file_type() == FileType::Mp4)
                .then(|| tag.pictures().first())
                .flatten()
        })
        .map(|picture| TagCover {
            mime_type: picture
                .mime_type()
                .map(|m| m.as_str())
                .unwrap_or("")
                .to_owned(),
            data: picture.data().to_vec(),
        });
    Ok(TrackTags {
        title: text(tag, ItemKey::TrackTitle).unwrap_or_default(),
        artist: text(tag, ItemKey::TrackArtist),
        album: text(tag, ItemKey::AlbumTitle),
        album_artist: text(tag, ItemKey::AlbumArtist),
        year: tag
            .get_string(ItemKey::RecordingDate)
            .or_else(|| tag.get_string(ItemKey::Year))
            .and_then(|date| date.get(..4))
            .and_then(|year| year.parse().ok()),
        track_no: tag.track(),
        track_total: tag.track_total(),
        disc_no: tag.disk(),
        genre: text(tag, ItemKey::Genre),
        lyrics: text(tag, ItemKey::Lyrics).or_else(|| text(tag, ItemKey::UnsyncLyrics)),
        comment: text(tag, ItemKey::Comment),
        isrc: text(tag, ItemKey::Isrc),
        cover,
        replay_gain_track_gain: text(tag, ItemKey::ReplayGainTrackGain),
        replay_gain_track_peak: text(tag, ItemKey::ReplayGainTrackPeak),
        r128_track_gain: tag
            .get_string(ItemKey::R128TrackGain)
            .and_then(|gain| gain.parse().ok()),
    })
}

fn set_text(tag: &mut Tag, key: ItemKey, value: Option<&str>) {
    tag.remove_key(key);
    if let Some(value) = value {
        tag.insert_text(key, value.to_owned());
    }
}

fn update(tag: &mut Tag, tags: &TrackTags, mp4: bool) -> CoreResult<()> {
    let fields = [
        (
            ItemKey::TrackTitle,
            (!tags.title.is_empty()).then_some(tags.title.as_str()),
        ),
        (ItemKey::TrackArtist, tags.artist.as_deref()),
        (ItemKey::AlbumTitle, tags.album.as_deref()),
        (ItemKey::AlbumArtist, tags.album_artist.as_deref()),
        (ItemKey::Genre, tags.genre.as_deref()),
        (ItemKey::Comment, tags.comment.as_deref()),
        (ItemKey::Isrc, tags.isrc.as_deref()),
        (
            ItemKey::ReplayGainTrackGain,
            tags.replay_gain_track_gain.as_deref(),
        ),
        (
            ItemKey::ReplayGainTrackPeak,
            tags.replay_gain_track_peak.as_deref(),
        ),
    ];
    for (key, value) in fields {
        if value.is_some_and(|s| s.is_empty() || s.contains('\0')) {
            return Err(CoreError::invalid("Tag textual vazia ou com caractere NUL"));
        }
        set_text(tag, key, value);
    }
    for (key, value) in [
        (ItemKey::RecordingDate, tags.year),
        (ItemKey::TrackNumber, tags.track_no),
        (ItemKey::TrackTotal, tags.track_total),
        (ItemKey::DiscNumber, tags.disc_no),
    ] {
        if value.is_some_and(|n| n == 0 || (mp4 && n > u16::MAX as u32)) {
            return Err(CoreError::invalid(
                "Ano/faixa/disco fora do intervalo suportado",
            ));
        }
        let value = value.map(|n| {
            if key == ItemKey::RecordingDate {
                format!("{n:04}")
            } else {
                n.to_string()
            }
        });
        set_text(tag, key, value.as_deref());
    }
    if tags.year.is_some_and(|n| n > 9999) {
        return Err(CoreError::invalid("Ano fora do intervalo 1–9999"));
    }
    tag.remove_key(ItemKey::Year);
    tag.remove_key(ItemKey::Lyrics);
    tag.remove_key(ItemKey::UnsyncLyrics);
    if let Some(lyrics) = &tags.lyrics {
        if lyrics.is_empty() || lyrics.contains('\0') {
            return Err(CoreError::invalid("Letra vazia ou com caractere NUL"));
        }
        let key = if tag.tag_type() == TagType::Id3v2 {
            ItemKey::UnsyncLyrics
        } else {
            ItemKey::Lyrics
        };
        set_text(tag, key, Some(lyrics));
    }
    set_text(
        tag,
        ItemKey::R128TrackGain,
        tags.r128_track_gain.map(|n| n.to_string()).as_deref(),
    );
    if mp4 {
        while !tag.pictures().is_empty() {
            tag.remove_picture(0);
        }
    } else {
        tag.remove_picture_type(PictureType::CoverFront);
    }
    if let Some(cover) = &tags.cover {
        let mut picture = Picture::from_reader(&mut Cursor::new(&cover.data)).map_err(tag_error)?;
        if picture.mime_type().map(|m| m.as_str()) != Some(cover.mime_type.as_str()) {
            return Err(CoreError::invalid("MIME da capa difere do conteúdo"));
        }
        picture.set_pic_type(PictureType::CoverFront);
        tag.push_picture(picture);
    }
    Ok(())
}

/// Mantém o original intacto em qualquer falha de escrita ou verificação.
pub fn write_tags(path: &Path, tags: &TrackTags) -> CoreResult<()> {
    if std::fs::metadata(path)?.permissions().readonly() {
        return Err(CoreError::coded("disk", "Arquivo de áudio somente leitura"));
    }
    let file = open(path)?;
    if tags.r128_track_gain.is_some() && file.file_type() != FileType::Opus {
        return Err(CoreError::invalid("R128_TRACK_GAIN é exclusivo de Opus"));
    }
    let mut tag = file
        .primary_tag()
        .cloned()
        .unwrap_or_else(|| Tag::new(file.primary_tag_type()));
    update(&mut tag, tags, file.file_type() == FileType::Mp4)?;
    let stage = tempfile::Builder::new()
        .prefix(".reverb-tags-")
        .suffix(&format!(
            ".{}",
            path.extension().unwrap_or_default().to_string_lossy()
        ))
        .tempfile_in(path.parent().unwrap_or_else(|| Path::new(".")))?;
    std::fs::copy(path, stage.path())?;
    tag.save_to_path(
        stage.path(),
        lofty::config::WriteOptions::new().use_id3v23(false),
    )
    .map_err(tag_error)?;
    if read_tags(stage.path())? != *tags {
        return Err(CoreError::coded(
            "tags",
            "Tags relidas diferem das tags solicitadas",
        ));
    }
    stage.as_file().sync_all()?;
    stage
        .persist(path)
        .map_err(|error| CoreError::from(error.error))?;
    Ok(())
}
