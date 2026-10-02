//! Identificação de metadados com nota de confiança (F08, arquitetura §11).

pub mod cache;
pub mod content_type;
pub mod deezer;
pub mod itunes;
pub mod model;
pub mod musicbrainz;
pub mod normalize;
pub mod official;
pub mod parse_title;
pub mod provider;
pub mod score;
pub mod service;

pub use content_type::{content_type, ContentType};
pub use model::{
    Bucket, MetadataFields, MetadataOverride, MetadataResult, PreviewRequest, ScoredCandidate,
};
pub use normalize::norm;
pub use official::{find_official, OfficialFound, OfficialMatch, OfficialVia};
pub use parse_title::{parse_title, ParsedTitle};
pub use provider::{Candidate, Endpoints, MetadataProvider, Query, RateLimiter};
pub use score::{score, DurationTolerance, Subject};
pub use service::{IdentifyInput, MetadataService, SourcePlan};

#[cfg(test)]
mod tests;
