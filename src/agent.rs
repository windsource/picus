use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use std::error::Error;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait AgentProvider {
    async fn start(&self) -> Result<(), Box<dyn Error>>;
    async fn stop(&self) -> Result<(), Box<dyn Error>>;
}
