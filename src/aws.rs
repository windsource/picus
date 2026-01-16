use crate::agent::{FilterLabels, Labels};
use crate::{agent::AgentProvider, env::read_env_or_exit};
use async_trait::async_trait;
use aws_config::BehaviorVersion;
use aws_sdk_ec2::types::InstanceStateName;
use aws_sdk_ec2::Client;
use log::{debug, error, info};
use std::error::Error;
use std::time::Duration;
use tokio::time::sleep;

pub struct AwsAgentProviderParams {
    instance_id: String,
    filter_labels: String,
}

impl AwsAgentProviderParams {
    pub fn from_env() -> AwsAgentProviderParams {
        AwsAgentProviderParams {
            instance_id: read_env_or_exit("PICUS_AWS_INSTANCE_ID"),
            filter_labels: read_env_or_exit("PICUS_AGENT_WOODPECKER_FILTER_LABELS"),
        }
    }
}

pub struct AwsAgentProvider {
    instance_id: String,
    client: Client,
    filter_labels: FilterLabels,
}

impl AwsAgentProvider {
    pub async fn new(params: AwsAgentProviderParams) -> Result<AwsAgentProvider, Box<dyn Error>> {
        let config = aws_config::defaults(BehaviorVersion::v2026_01_12())
            .load()
            .await;
        let client = Client::new(&config);

        let ap = AwsAgentProvider {
            instance_id: params.instance_id,
            client,
            filter_labels: FilterLabels::from_string(&params.filter_labels),
        };

        // Check access to AWS and if instance exists
        match ap.get_instance_state().await {
            Err(e) => {
                error!("{e:?}");
                return Err(e);
            }
            Ok(state_name) => info!(
                "Acces to AWS ok and instance {} found, state {:?}",
                ap.instance_id, state_name
            ),
        }

        Ok(ap)
    }

    async fn get_instance_state(&self) -> Result<InstanceStateName, Box<dyn Error>> {
        let ids = Some(vec![self.instance_id.clone()]);
        let resp = self
            .client
            .describe_instances()
            .set_instance_ids(ids)
            .send()
            .await?;

        for reservation in resp.reservations() {
            for instance in reservation.instances() {
                if instance.instance_id().unwrap() == self.instance_id {
                    match instance.state() {
                        Some(state) => match &state.name {
                            Some(name) => return Ok(name.clone()),
                            None => {
                                return Err(format!(
                                    "could not resolve name state of instance {}",
                                    self.instance_id
                                )
                                .into())
                            }
                        },
                        None => {
                            return Err(format!(
                                "could not get instance state of {}",
                                self.instance_id
                            )
                            .into())
                        }
                    }
                }
            }
        }

        Err(format!("instance {} not found", self.instance_id).into())
    }
}

#[async_trait]
impl AgentProvider for AwsAgentProvider {
    async fn start(&self) -> Result<(), Box<dyn Error>> {
        let state = self.get_instance_state().await?;
        match state {
            InstanceStateName::Stopped => {
                info!("Starting instance ...");
                self.client
                    .start_instances()
                    .instance_ids(self.instance_id.clone())
                    .send()
                    .await?;
            }
            InstanceStateName::Stopping => {
                info!("Instance is stopping. Need to wait until it is stopped to start it again.");
                let max_iterations = 150;
                for _ in 0..max_iterations {
                    sleep(Duration::from_secs(2)).await;
                    let state = self.get_instance_state().await?;
                    match state {
                        InstanceStateName::Stopping => {
                            debug!("Instance still stopping. Continue waiting ...")
                        }
                        InstanceStateName::Stopped => {
                            info!("Instance is stopped. Starting instance ...");
                            self.client
                                .start_instances()
                                .instance_ids(self.instance_id.clone())
                                .send()
                                .await?;
                            break;
                        }
                        InstanceStateName::Pending | InstanceStateName::Running => {
                            info!("Instance state is already {state:?}. No need to start it.");
                            break;
                        }
                        _ => {
                            error!("Unexpected instance state {state:?}. Cannot start instance.");
                            return Err("no instance to start".into());
                        }
                    }
                }
                error!("Timeout reached to stop server! Cannot start it");
                return Err("starting instance failed".into());
            }
            InstanceStateName::Pending | InstanceStateName::Running => {
                info!("Instance state is already {state:?}. No need to start it.")
            }
            InstanceStateName::ShuttingDown | InstanceStateName::Terminated => {
                error!("Instance is {state:?}. Cannot start it.");
                return Err("no instance to start".into());
            }
            _ => {
                error!("Instance is in an unknown state: {state:?}. Cannot start it.");
                return Err("no instance to start".into());
            }
        }
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn Error>> {
        let state = self.get_instance_state().await?;
        match state {
            InstanceStateName::Running => {
                info!("Stopping instance ...");
                self.client
                    .stop_instances()
                    .instance_ids(self.instance_id.clone())
                    .send()
                    .await?;
            }
            InstanceStateName::Pending => {
                info!("Instance is state is pending. Need to wait until it is running to stop it.");
                let max_iterations = 150;
                for _ in 0..max_iterations {
                    sleep(Duration::from_secs(2)).await;
                    let state = self.get_instance_state().await?;
                    match state {
                        InstanceStateName::Pending => {
                            debug!("Instance still Pending. Continue waiting ...")
                        }
                        InstanceStateName::Running => {
                            info!("Instance is now running. Stopping instance ...");
                            self.client
                                .stop_instances()
                                .instance_ids(self.instance_id.clone())
                                .send()
                                .await?;
                            break;
                        }
                        InstanceStateName::Stopping | InstanceStateName::Stopped => {
                            info!("Instance state is already {state:?}. No need to stop it.");
                            break;
                        }
                        _ => {
                            error!("Unexpected instance state {state:?}. Cannot stop instance.");
                            return Err("no instance to start".into());
                        }
                    }
                }
            }
            InstanceStateName::Stopping | InstanceStateName::Stopped => {
                info!("Instance state is already {state:?}. No need to stop it.")
            }
            _ => {
                error!("Instance is in an unknown state: {state:?}. Cannot stop it.");
                return Err("No instance to start!".into());
            }
        }
        Ok(())
    }

    async fn is_running(&self) -> Result<bool, Box<dyn Error>> {
        let state = self.get_instance_state().await?;
        match state {
            InstanceStateName::Running => Ok(true),
            _ => Ok(false),
        }
    }

    fn supports_labels(&self, labels: &Labels) -> bool {
        self.filter_labels.supports(labels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use test_log::test;

    #[test(tokio::test)]
    #[ignore]
    async fn aws_start_and_stop() {
        let params = AwsAgentProviderParams::from_env();
        let ap = AwsAgentProvider::new(params).await.unwrap();
        assert!(ap.start().await.is_ok());
        assert!(ap.stop().await.is_ok());
    }
}
