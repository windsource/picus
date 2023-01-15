use crate::{agent::AgentProvider, env::read_env_or_exit};
use async_trait::async_trait;
use aws_sdk_ec2::{model::InstanceStateName, Client};
use log::{error, info};
use std::error::Error;

pub struct AwsAgentProviderParams {
    instance_id: String,
}

impl AwsAgentProviderParams {
    pub fn from_env() -> AwsAgentProviderParams {
        AwsAgentProviderParams {
            instance_id: read_env_or_exit("PICUS_AWS_INSTANCE_ID"),
        }
    }
}

pub struct AwsAgentProvider {
    params: AwsAgentProviderParams,
    client: Client,
}

impl AwsAgentProvider {
    pub async fn new(params: AwsAgentProviderParams) -> Result<AwsAgentProvider, Box<dyn Error>> {
        let config = aws_config::from_env().load().await;
        let client = Client::new(&config);

        let ap = AwsAgentProvider { params, client };

        // Check access to AWS and if instance exists
        match ap.get_instance_state().await {
            Err(e) => {
                error!("{e:?}");
                return Err(e);
            }
            Ok(state_name) => info!(
                "Acces to AWS ok and instance {} found, state {:?}",
                ap.params.instance_id, state_name
            ),
        }

        Ok(ap)
    }

    async fn get_instance_state(&self) -> Result<InstanceStateName, Box<dyn Error>> {
        let ids = Some(vec![self.params.instance_id.clone()]);
        let resp = self
            .client
            .describe_instances()
            .set_instance_ids(ids)
            .send()
            .await?;

        for reservation in resp.reservations().unwrap_or_default() {
            for instance in reservation.instances().unwrap_or_default() {
                if instance.instance_id().unwrap() == self.params.instance_id {
                    match instance.state() {
                        Some(state) => match &state.name {
                            Some(name) => return Ok(name.clone()),
                            None => {
                                return Err(format!(
                                    "Could not resolve name state of instance {}",
                                    self.params.instance_id
                                )
                                .into())
                            }
                        },
                        None => {
                            return Err(format!(
                                "Could not get instance state of {}",
                                self.params.instance_id
                            )
                            .into())
                        }
                    }
                }
            }
        }

        Err(format!("Instance {} not found", self.params.instance_id).into())
    }
}

#[async_trait]
impl AgentProvider for AwsAgentProvider {
    async fn start(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn Error>> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test(tokio::test)]
    #[ignore]
    async fn new_aws_agent() {
        let params = AwsAgentProviderParams::from_env();
        let res = AwsAgentProvider::new(params).await;
        assert!(res.is_ok());
    }
}
