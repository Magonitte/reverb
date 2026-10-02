//! Organização de arquivos: nomes portáveis e publicação atômica na biblioteca.

mod moving;
pub mod sanitize;
pub mod template;

pub use moving::{move_into_library, move_into_library_with_mode, MoveMode};
pub use sanitize::{sanitize_component, sanitize_path, unique_path};
