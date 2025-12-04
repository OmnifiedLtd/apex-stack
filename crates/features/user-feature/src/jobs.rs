use serde::{Deserialize, Serialize};
use sqlxmq::{job, CurrentJob, JobRegistry};
use tracing::info;
use uuid::Uuid;

/// Arguments for the welcome email job
#[derive(Debug, Serialize, Deserialize)]
pub struct WelcomeEmailArgs {
    pub user_id: Uuid,
    pub email: String,
    pub name: String,
}

/// Send a welcome email to a newly registered user
#[job(channel_name = "emails")]
pub async fn send_welcome_email(
    mut current_job: CurrentJob,
) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
    // Extract arguments from the job payload
    let args: WelcomeEmailArgs = current_job.json()?.expect("job arguments");

    info!(
        user_id = %args.user_id,
        email = %args.email,
        name = %args.name,
        "Sending welcome email"
    );

    // In a real application, you would call an email service here
    // For example: email_client.send_welcome(args.email, args.name).await?;

    current_job.complete().await?;
    Ok(())
}

/// Registry of all user-related jobs
pub struct UserJobs;

impl UserJobs {
    /// Create a job registry containing all user feature jobs
    pub fn registry() -> JobRegistry {
        JobRegistry::new(&[send_welcome_email])
    }

    /// Spawn a welcome email job within a transaction
    pub async fn enqueue_welcome_email(
        tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        user_id: Uuid,
        email: String,
        name: String,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let args = WelcomeEmailArgs {
            user_id,
            email,
            name,
        };

        send_welcome_email
            .builder()
            .set_json(&args)?
            .spawn(&mut **tx)
            .await?;

        Ok(())
    }
}
