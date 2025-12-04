use thiserror::Error;

#[derive(Error, Debug)]
pub enum UserFeatureError {
    #[error("Domain error: {0}")]
    Domain(#[from] domain::DomainError),

    #[error("Queue error: {0}")]
    Queue(String),

    #[error("User not found: {0}")]
    NotFound(uuid::Uuid),

    #[error("Email already exists: {0}")]
    EmailExists(String),
}
