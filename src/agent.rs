use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use std::error::Error;
use crate::env::*;

pub struct AgentConfig {
    pub server: String,
    pub agent_secret: String,
    pub agent_image: String,
    pub grpc_secure: String
}

impl AgentConfig {
    pub fn from_env() -> AgentConfig {
        AgentConfig {
            server: read_env_or_exit("PICUS_AGENT_WOODPECKER_SERVER"),
            agent_secret: read_env_or_exit("PICUS_AGENT_WOODPECKER_AGENT_SECRET"),
            agent_image: read_env_or_default("PICUS_AGENT_IMAGE", "woodpeckerci/woodpecker-agent:latest"),
            grpc_secure: read_env_or_default("PICUS_AGENT_WOODPECKER_GRPC_SECURE", "true"),
        }
    }
}

#[cfg_attr(test, automock)]
#[async_trait]
pub trait AgentProvider {
    async fn start(&self) -> Result<(), Box<dyn Error>>;
    async fn stop(&self) -> Result<(), Box<dyn Error>>;
}