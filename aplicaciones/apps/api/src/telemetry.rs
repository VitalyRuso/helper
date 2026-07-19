use anyhow::Result;
use tracing_subscriber::{fmt, EnvFilter};

pub fn init(level: &str) -> Result<()> {
    let filter = EnvFilter::try_new(level).unwrap_or_else(|_| EnvFilter::new("info"));
    fmt().with_env_filter(filter).compact().init();
    Ok(())
}
