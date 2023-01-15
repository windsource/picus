use agent::AgentConfig;
use env::*;
use env_logger::Env;
use go_parse_duration::parse_duration;
use hetzner::HetznerAgentProviderParams;
use log::info;
use reqwest::Error;
use std::time::Duration;
use tokio::time::sleep;

mod strategy;
use strategy::*;

mod agent;
mod aws;
mod env;
mod hetzner;

fn duration_from_string(duration_string: &str) -> Duration {
    Duration::from_nanos(parse_duration(duration_string).unwrap().try_into().unwrap())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();
    info!(
        "Starting {} version {}",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION")
    );

    let wp_token = read_env_or_exit("PICUS_WOODPECKER_TOKEN");
    let wp_server = read_env_or_exit("PICUS_WOODPECKER_SERVER");
    let poll_interval = duration_from_string(&read_env_or_default("PICUS_POLL_INTERVAL", "10s"));
    let shutdown_timer = duration_from_string(&read_env_or_default("PICUS_MAX_IDLE_TIME", "30m"));

    let agent_config = AgentConfig::from_env();
    let hetzner_params = HetznerAgentProviderParams::from_env();
    let hetzner_agent_provider = hetzner::HetznerAgentProvider::new(hetzner_params, agent_config);
    let mut strategy = Strategy::new(Box::new(hetzner_agent_provider), shutdown_timer);

    let request_url = format!("{}/api/queue/info", wp_server);
    let client = reqwest::Client::new();

    loop {
        let response = client
            .get(&request_url)
            .bearer_auth(&wp_token)
            .send()
            .await?;

        let wp_queue_info: WpQueueInfo = response.json().await?;
        info!("{:?}", wp_queue_info);

        strategy.apply(&wp_queue_info).await;

        sleep(poll_interval).await;
    }
}
