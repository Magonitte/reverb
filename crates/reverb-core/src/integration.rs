//! Validated external entry points. No OS dependencies or side effects.

use crate::profiles::profile;
use crate::urlkind::{classify, UrlKind};
use crate::{CoreError, CoreResult};
use url::Url;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeepLinkAction {
    Open,
    Add {
        url: String,
        profile_id: Option<String>,
    },
}

pub fn supported_url(input: &str) -> Option<String> {
    match classify(input) {
        UrlKind::Video { url, .. } | UrlKind::Collection { url } => Some(url),
        _ => None,
    }
}

#[derive(Default)]
pub struct ClipboardDedup {
    last: Option<String>,
}
impl ClipboardDedup {
    pub fn observe(&mut self, text: &str) -> Option<String> {
        let url = supported_url(text)?;
        if self.last.as_ref() == Some(&url) {
            return None;
        }
        self.last = Some(url.clone());
        Some(url)
    }
}

pub fn add_link(input: &str) -> CoreResult<String> {
    let url = supported_url(input).ok_or_else(|| CoreError::invalid("unsupported URL"))?;
    let mut link =
        Url::parse("reverb://add").map_err(|error| CoreError::Internal(error.to_string()))?;
    link.query_pairs_mut().append_pair("url", &url);
    Ok(link.to_string())
}

pub fn parse_deep_link(input: &str) -> CoreResult<DeepLinkAction> {
    let link = Url::parse(input).map_err(|_| CoreError::invalid("invalid deep link"))?;
    if link.scheme() != "reverb"
        || !link.username().is_empty()
        || link.password().is_some()
        || link.port().is_some()
        || !matches!(link.path(), "" | "/")
        || link.fragment().is_some()
    {
        return Err(CoreError::invalid("invalid deep link"));
    }
    match link.host_str() {
        Some("open") if link.query().is_none() => Ok(DeepLinkAction::Open),
        Some("add") => {
            let mut target = None;
            let mut profile_id = None;
            for (key, value) in link.query_pairs() {
                match key.as_ref() {
                    "url" if target.is_none() => target = Some(value.into_owned()),
                    "profile" if profile_id.is_none() => profile_id = Some(value.into_owned()),
                    _ => return Err(CoreError::invalid("invalid deep link parameters")),
                }
            }
            let target = target.ok_or_else(|| CoreError::invalid("missing deep link URL"))?;
            let url = match classify(&target) {
                UrlKind::Video { url, .. } | UrlKind::Collection { url } => url,
                _ => return Err(CoreError::invalid("unsupported deep link URL")),
            };
            if profile_id.as_ref().is_some_and(|id| profile(id).is_none()) {
                return Err(CoreError::invalid("unknown deep link profile"));
            }
            Ok(DeepLinkAction::Add { url, profile_id })
        }
        _ => Err(CoreError::invalid("unknown deep link action")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clipboard_deduplicates_canonical_urls_and_ignores_text() {
        let mut watcher = ClipboardDedup::default();
        assert!(watcher.observe("ordinary text").is_none());
        assert!(watcher.observe("https://youtu.be/jNQXAC9IVRw").is_some());
        assert!(watcher
            .observe("https://www.youtube.com/watch?v=jNQXAC9IVRw&t=2")
            .is_none());
        assert!(watcher.observe("ordinary text").is_none());
        assert!(watcher.observe("https://youtu.be/jNQXAC9IVRw").is_none());
        assert!(watcher.observe("https://youtu.be/dQw4w9WgXcQ").is_some());
    }

    #[test]
    fn copied_links_use_the_same_validated_entry_point() {
        let link = add_link("https://youtu.be/jNQXAC9IVRw?t=2").unwrap();
        assert!(matches!(
            parse_deep_link(&link).unwrap(),
            DeepLinkAction::Add { .. }
        ));
        assert!(add_link("ordinary text").is_err());
        assert!(add_link("https://evil.example").is_err());
    }

    #[test]
    fn deep_links_validate_and_decode_external_input() {
        let encoded = "reverb://add?url=https%3A%2F%2Fyoutu.be%2FjNQXAC9IVRw";
        assert_eq!(
            parse_deep_link(encoded).unwrap(),
            DeepLinkAction::Add {
                url: "https://www.youtube.com/watch?v=jNQXAC9IVRw".into(),
                profile_id: None,
            }
        );
        assert_eq!(
            parse_deep_link(&format!("{encoded}&profile=mp3_320")).unwrap(),
            DeepLinkAction::Add {
                url: "https://www.youtube.com/watch?v=jNQXAC9IVRw".into(),
                profile_id: Some("mp3_320".into()),
            }
        );
        assert_eq!(
            parse_deep_link("reverb://open").unwrap(),
            DeepLinkAction::Open
        );
        assert!(matches!(
            parse_deep_link(
                "reverb://add?url=https%3A%2F%2Fwww.youtube.com%2Fplaylist%3Flist%3DPL123"
            )
            .unwrap(),
            DeepLinkAction::Add { .. }
        ));
        for invalid in [
            "reverb://add",
            "reverb://add?url=",
            "https://open",
            "reverb://unknown",
            "reverb://add?url=javascript%3Aalert(1)",
            "reverb://add?url=rick+astley",
            "reverb://add?url=https%3A%2F%2Fyoutube.com.evil.com%2Fwatch%3Fv%3DjNQXAC9IVRw",
            "reverb://open/path",
            "reverb://open?url=x",
            "reverb://user@open",
            "reverb://open#fragment",
            "reverb://open:123",
        ] {
            assert!(parse_deep_link(invalid).is_err(), "{invalid}");
        }
        for suffix in [
            "&profile=bogus",
            "&profile=",
            "&url=x",
            "&profile=original&profile=mp3_320",
        ] {
            assert!(
                parse_deep_link(&format!("{encoded}{suffix}")).is_err(),
                "{suffix}"
            );
        }
    }
}
