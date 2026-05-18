mod error;
mod model;
mod store;

pub use error::AppError;
pub use model::{AppConfig, Note, NoteMetadata, SaveNoteRequest};
pub use store::default_store;
