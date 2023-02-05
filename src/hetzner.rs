use crate::agent::AgentProvider;
use crate::env::*;
use async_trait::async_trait;
use handlebars::Handlebars;
use hcloud::apis::configuration::Configuration;
use hcloud::apis::servers_api::{
    self, CreateServerParams, DeleteServerParams, GetServerParams, ListServersError,
    ListServersParams, ShutdownServerParams,
};
use hcloud::models::*;
use log::{debug, error, info};
use serde::Serialize;
use std::collections::hash_map::RandomState;
use std::collections::HashMap;
use std::error::Error;
use std::process;
use std::time::Duration;
use tokio::time::sleep;

static SERVICE_NAME: &str = "picus";

static USER_DATA_TEMPLATE: &str = r#"#cloud-config
write_files:
- content: |
    # docker-compose.yml
    version: '3'

    services:

      woodpecker-agent:
        image: {{ image }}
        command: agent
        restart: always
        volumes:
          - /var/run/docker.sock:/var/run/docker.sock
        environment:
          {{#each params}}
          - {{ this.0 }}={{ this.1 }}
          {{/each}}
  path: /root/docker-compose.yml
runcmd:
- [ sh, -xc, "cd /root; docker run --rm --privileged multiarch/qemu-user-static --reset -p yes; docker compose up -d" ]
"#;

fn create_user_data(agent_config: &AgentConfig) -> String {
    let mut handlebars = Handlebars::new();
    handlebars.register_escape_fn(handlebars::no_escape);

    assert!(handlebars
        .register_template_string("t1", USER_DATA_TEMPLATE)
        .is_ok());

    handlebars.render("t1", &agent_config).unwrap()
}
pub struct HetznerAgentProviderParams {
    hcloud_config: HcloudParams,
    agent_config: AgentConfig,
}

struct HcloudParams {
    api_token: String,
    server_type: String,
    location: String,
    /// List of ssh key for access to the servers separated by comma
    ssh_keys: String,
    /// Unique string according to RFC 1123 to identify resources created in
    /// Hetzner cloud for this service
    id: String,
}

#[derive(Serialize)]
struct AgentConfig {
    image: String,
    /// (key, value)
    params: Vec<(String, String)>,
}

impl HetznerAgentProviderParams {
    pub fn from_env() -> HetznerAgentProviderParams {
        HetznerAgentProviderParams {
            hcloud_config: HcloudParams {
                api_token: read_env_or_exit("PICUS_HCLOUD_TOKEN"),
                server_type: read_env_or_default("PICUS_HCLOUD_SERVER_TYPE", "cx11"),
                location: read_env_or_default("PICUS_HCLOUD_LOCATION", "nbg1"),
                ssh_keys: read_env_or_exit("PICUS_HCLOUD_SSH_KEYS"),
                id: read_env_or_default("PICUS_HCLOUD_ID", "picus-test"),
            },
            agent_config: AgentConfig {
                image: read_env_or_default(
                    "PICUS_AGENT_IMAGE",
                    "woodpeckerci/woodpecker-agent:latest",
                ),
                params: {
                    let mut params = Vec::new();
                    let mut all_keys = Vec::new();

                    for (key, value) in std::env::vars() {
                        if key.starts_with("PICUS_AGENT_WOODPECKER_") {
                            debug!("{} found", key);
                            params.push((
                                key.strip_prefix("PICUS_AGENT_").unwrap().to_string(),
                                value,
                            ));
                            all_keys.push(key);
                        }
                    }

                    // Check if the core parametes are present
                    let required_keys = [
                        "PICUS_AGENT_WOODPECKER_SERVER",
                        "PICUS_AGENT_WOODPECKER_AGENT_SECRET",
                    ];
                    required_keys.iter().for_each(|k| {
                        if !all_keys.contains(&k.to_string()) {
                            error!("Environment variable {} not found!", *k);
                            process::exit(1);
                        }
                    });

                    params
                },
            },
        }
    }
}

pub struct HetznerAgentProvider {
    params: HcloudParams,
    config: Configuration,
    labels: HashMap<String, String, RandomState>,
    label_selector: String,
    server_name: String,
    ssh_keys: Vec<String>,
    user_data: String,
}

impl HetznerAgentProvider {
    pub fn new(params: HetznerAgentProviderParams) -> HetznerAgentProvider {
        let mut configuration = Configuration::new();
        configuration.bearer_access_token = Some(params.hcloud_config.api_token.clone());

        let server_name = format!("{}-{}", SERVICE_NAME, params.hcloud_config.id);
        assert!(server_name.len() <= 63);

        let mut ssh_keys: Vec<String> = Vec::new();
        let iter = params.hcloud_config.ssh_keys.split(',');
        for s in iter {
            ssh_keys.push(s.to_string());
        }

        let label_selector = format!("{}=={}", SERVICE_NAME, params.hcloud_config.id);

        let mut labels = HashMap::new();
        labels.insert(SERVICE_NAME.to_string(), params.hcloud_config.id.clone());

        HetznerAgentProvider {
            params: params.hcloud_config,
            config: configuration,
            labels,
            label_selector,
            server_name,
            ssh_keys,
            user_data: create_user_data(&params.agent_config),
        }
    }

    /// Returns existsing server instances for this service
    async fn list_instances(
        &self,
    ) -> Result<ListServersResponse, hcloud::apis::Error<ListServersError>> {
        let params = ListServersParams {
            label_selector: Some(self.label_selector.clone()),
            ..Default::default()
        };
        servers_api::list_servers(&self.config, params).await
    }

    async fn create_server_from_scratch(&self) -> Result<(), String> {
        // start server
        let params = CreateServerParams {
            create_server_request: Some(CreateServerRequest {
                firewalls: None,
                image: "docker-ce".to_string(),
                labels: Some(self.labels.clone()),
                location: Some(self.params.location.clone()),
                name: self.server_name.clone(),
                server_type: self.params.server_type.clone(),
                ssh_keys: Some(self.ssh_keys.clone()),
                start_after_create: Some(true),
                user_data: Some(self.user_data.clone()),
                ..Default::default()
            }),
        };

        let result = servers_api::create_server(&self.config, params).await;

        if let Err(err) = result {
            let msg = format!("Error: Could not create server from scratch: {}", err);
            error!("{}", msg);
            return Err(msg);
        }
        Ok(())
    }

    /// Shutdown server and wait until shutdown is finished
    async fn shutdown_server(&self, id: i32) -> Result<(), String> {
        let result = servers_api::shutdown_server(&self.config, ShutdownServerParams { id }).await;
        if let Err(err) = result {
            let msg = format!("Error: Could not shutdown server: {}", err);
            error!("{}", msg);
            return Err(msg);
        }

        let max_iterations = 60;
        for _ in 0..max_iterations {
            sleep(Duration::from_secs(10)).await;

            let result = servers_api::get_server(&self.config, GetServerParams { id }).await;
            if let Err(err) = result {
                let msg = format!("Error: Could not get server: {}", err);
                error!("{}", msg);
                return Err(msg);
            }

            if let Some(server) = result.unwrap().server {
                if server.status == server::Status::Off {
                    return Ok(());
                }
            } else {
                return Err("Server not found anymore.".to_string());
            }
        }
        Err("Error: Timeout when shutting down server".to_string())
    }
}

#[async_trait]
impl AgentProvider for HetznerAgentProvider {
    async fn start(&self) -> Result<(), Box<dyn Error>> {
        let servers = self.list_instances().await?.servers;
        if !servers.is_empty() {
            info!(
                "Already {} servers existing. No need to start an other one.",
                servers.len()
            );
        } else {
            info!("Starting server ...");
            self.create_server_from_scratch().await?
        }
        Ok(())
    }

    async fn stop(&self) -> Result<(), Box<dyn Error>> {
        let servers = self.list_instances().await?.servers;
        if servers.is_empty() {
            info!("No server found which needs to be shutdown.");
        } else {
            for server in servers {
                let id = server.id;
                if server.status == server::Status::Running {
                    info!("Shutting down server {}", id);
                    let _ = self.shutdown_server(id).await;
                }
                info!("Deleting server {}", id);
                servers_api::delete_server(&self.config, DeleteServerParams { id }).await?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn hcloud_start_and_stop() {
        let params = HetznerAgentProviderParams::from_env();
        let ap = HetznerAgentProvider::new(params);

        assert!(ap.start().await.is_ok());

        assert!(!ap.list_instances().await.unwrap().servers.is_empty());

        // Wait some time for server being up and running
        sleep(Duration::from_secs(30)).await;

        assert!(ap.stop().await.is_ok());

        assert!(ap.list_instances().await.unwrap().servers.is_empty());
    }
}
