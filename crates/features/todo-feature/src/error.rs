use thiserror::Error;

#[derive(Error, Debug)]
pub enum TodoFeatureError {
    #[error("Domain error: {0}")]
    Domain(#[from] domain::DomainError),

    #[error("Todo not found: {0}")]
    NotFound(uuid::Uuid),

    #[error("User not found: {0}")]
    UserNotFound(uuid::Uuid),
}
