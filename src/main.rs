use crate::{
    aws::{AwsAgentProvider, AwsAgentProviderParams},
    env::{read_env_or_default, read_env_or_exit},
    hetzner::{HetznerAgentProvider, HetznerAgentProviderParams},
};
use agent::AgentProvider;
use env_logger::Env;
use go_parse_duration::parse_duration;
use log::{error, info};
use reqwest::Error;
use std::{process, time::Duration};
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
    let provider_type = read_env_or_exit("PICUS_PROVIDER_TYPE");
    let agent_id = read_env_or_exit("PICUS_WOODPECKER_AGENT_ID");

    let agent_provider: Box<dyn AgentProvider>;
    match provider_type.as_str() {
        "hcloud" => {
            let params = HetznerAgentProviderParams::from_env();
            agent_provider = Box::new(HetznerAgentProvider::new(params));
        }
        "aws" => {
            let params = AwsAgentProviderParams::from_env();
            let res = AwsAgentProvider::new(params).await;
            match res {
                Ok(ap) => agent_provider = Box::new(ap),
                Err(e) => {
                    error!("Could not create agent provider for AWS: {e}");
                    process::exit(1);
                }
            }
        }
        _ => {
            error!("Unkown provider type {provider_type}");
            process::exit(1);
        }
    }

    let mut strategy = Strategy::new(agent_provider, shutdown_timer, agent_id);

    let request_url = format!("{}/api/queue/info", wp_server);
    let client = reqwest::Client::new();

    loop {
        let content = client
            .get(&request_url)
            .bearer_auth(&wp_token)
            .send()
            .await?
            .bytes()
            .await?;
        let Ok(wp_queue_info) = serde_json::from_slice::<WpQueueInfo>(&content) else {
            error!("Cannot decode JSON from {request_url}: {content:?}");
            process::exit(1);
        };

        strategy.apply(&wp_queue_info).await;

        sleep(poll_interval).await;
    }
}
