//! Fila persistente de downloads (F04): modelo, repositório, agendador, retry e autocura.

pub mod heal;
pub mod model;
pub mod repo;
pub mod runner;
pub mod scheduler;

#[cfg(test)]
pub(crate) mod fake;
#[cfg(test)]
mod tests;

pub use heal::{HealCoordinator, HealOutcome, HealTools, ToolsHeal};
pub use model::{
    DuplicateHit, EnqueueRequest, Job, JobOptions, JobStage, JobStatus, MoveTarget, PlaylistCtx,
    QueueState,
};
pub use runner::{PipelineRunner, ToolsPipeline};
pub use scheduler::{QueueDeps, QueueService};
