// === EmailProvider

/**
 * Swappable email transport. Handlers depend on this trait through `AppState`, never on a concrete provider.
 */
#[async_trait::async_trait]
pub trait EmailProvider: Send + Sync {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailError>;
}

#[derive(Debug)]
pub struct EmailError(pub String);

impl std::fmt::Display for EmailError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        return write!(f, "email send failed: {}", self.0);
    }
}

impl std::error::Error for EmailError {}

// === LogEmailProvider

/**
 * Dev/test provider.
 * Logs instead of sending, so signup and other flows that depend on `EmailProvider` work end to end with no external service.
 */
pub struct LogEmailProvider;

#[async_trait::async_trait]
impl EmailProvider for LogEmailProvider {
    async fn send(&self, to: &str, subject: &str, body: &str) -> Result<(), EmailError> {
        println!("[log-email-provider] to={} subject={}\n{}", to, subject, body);
        return Ok(());
    }
}
