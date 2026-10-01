//! Organização de arquivos: sanitização de nomes (F03) e, depois, o modelo de pastas (F09).

pub mod sanitize;

pub use sanitize::{sanitize_component, sanitize_path, unique_path};
