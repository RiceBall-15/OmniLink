use anyhow::Result;
use tokio::join;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Run all services concurrently
    join!(
        im_gateway::main::run(),
        im_api::main::run(),
        ai_service::main::run(),
        user_service::main::run(),
        file_service::main::run(),
        usage_service::main::run(),
        push_service::main::run(),
        config_service::main::run(),
    );

    Ok(())
}