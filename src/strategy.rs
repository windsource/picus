use crate::agent::{AgentProvider, Labels};
use log::{debug, error, info};
use serde::Deserialize;
use std::time::Duration;
use tokio::time::Instant;

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct WpQueueInfoPending {
    id: String,
    labels: Labels,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct WpQueueInfoRunning {
    id: String,
    labels: Labels,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct WpQueueInfoStats {
    worker_count: u32,
    pending_count: u32,
    waiting_on_deps_count: u32,
    running_count: u32,
    completed_count: u32,
}

#[derive(Deserialize, Debug)]
#[allow(dead_code)]
pub struct WpQueueInfo {
    pending: Option<Vec<WpQueueInfoPending>>,
    running: Option<Vec<WpQueueInfoRunning>>,
    stats: WpQueueInfoStats,
}

pub struct Strategy {
    agent_provider: Box<dyn AgentProvider>,
    last_time_running_agent: Option<Instant>,
    idle_time_before_stop: Duration,
    agent_id: String,
}

impl Strategy {
    pub fn new(
        agent_provider: Box<dyn AgentProvider>,
        idle_time_before_stop: Duration,
        agent_id: String,
    ) -> Strategy {
        Strategy {
            agent_provider,
            last_time_running_agent: None,
            idle_time_before_stop,
            agent_id,
        }
    }

    pub async fn apply(&mut self, queue_info: &WpQueueInfo) {
        info!("{:?}", queue_info.stats);

        // Check if agent is running
        let agent_is_running = self.agent_provider.is_running().await;
        if let Err(err) = agent_is_running {
            error!("Could no determine agent status: {}", err);
            return;
        }
        let agent_is_running = agent_is_running.unwrap();

        // Check for pending jobs that fit to the agent
        let mut pending_count = 0;
        if let Some(pending) = &queue_info.pending {
            info!("{:?}", pending);
            pending_count = pending
                .iter()
                .filter(|p| self.agent_provider.supports_labels(&p.labels))
                .collect::<Vec<_>>()
                .len();
        }

        // Check for running jobs on that agent
        let mut running_count = 0;
        if let Some(running) = &queue_info.running {
            running_count = running
                .iter()
                .filter(|r| r.id == self.agent_id)
                .collect::<Vec<_>>()
                .len();
        }

        info!(
            "Agent is {}running with {} jobs and {} pending jobs for that agent",
            if agent_is_running { "" } else { "not " },
            running_count,
            pending_count
        );

        if !agent_is_running && running_count == 0 && pending_count > 0 {
            info!("{} pending jobs. Starting agent.", pending_count);
            let result = self.agent_provider.start().await;
            if let Err(err) = result {
                error!("AgentProvider could not start server: {}", err);
            }
            self.last_time_running_agent = Some(Instant::now());
        }
        if running_count > 0 {
            self.last_time_running_agent = Some(Instant::now());
        }
        if agent_is_running && running_count == 0 && pending_count == 0 {
            if let Some(last_time_running_agent) = self.last_time_running_agent {
                if last_time_running_agent.elapsed() > self.idle_time_before_stop {
                    info!("Idle timeout reached. Stopping agent.");
                    let _ = self.agent_provider.stop().await;
                } else {
                    let remaining_lifetime =
                        (self.idle_time_before_stop - last_time_running_agent.elapsed()).as_secs();
                    info!(
                        "Remaining idle time before stop: {}m{}s",
                        remaining_lifetime / 60,
                        remaining_lifetime % 60
                    );
                }
            } else {
                info!("Still agent running from past. Stopping it.");
                let _ = self.agent_provider.stop().await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::MockAgentProvider;
    #[cfg(feature = "json")]
    use serde_json;
    use std::collections::HashMap;
    use tokio::time::sleep;

    #[test]
    fn parse_queue_info_all_null() {
        static QUEUE_INFO: &str = r#"{
  "pending": null,
  "waiting_on_deps": null,
  "running": null,
  "stats": {
    "worker_count": 0,
    "pending_count": 0,
    "waiting_on_deps_count": 0,
    "running_count": 0,
    "completed_count": 0
  },
  "paused": false
}"#;
        let wp_queue_info: WpQueueInfo = serde_json::from_str(QUEUE_INFO).unwrap();
        assert!(wp_queue_info.pending.is_none());
        assert_eq!(wp_queue_info.stats.worker_count, 0);
    }

    #[test]
    fn parse_queue_info_pending() {
        static QUEUE_INFO: &str = r#"{
  "pending": [
    {
      "id": "11",
      "data": "REDACTED",
      "labels": {
        "platform": "",
        "repo": "windsource/woodpecker-test"
      },
      "dependencies": null,
      "run_on": null,
      "dep_status": {},
      "agent_id": 0
    },
    {
      "id": "12",
      "data": "REDACTED",
      "labels": {
        "platform": "",
        "repo": "windsource/woodpecker-test"
      },
      "dependencies": null,
      "run_on": null,
      "dep_status": {},
      "agent_id": 0
    }
  ],
  "waiting_on_deps": null,
  "running": null,
  "stats": {
    "worker_count": 0,
    "pending_count": 2,
    "waiting_on_deps_count": 0,
    "running_count": 0,
    "completed_count": 0
  },
  "paused": false
}"#;
        let wp_queue_info: WpQueueInfo = serde_json::from_str(QUEUE_INFO).unwrap();
        let pending = wp_queue_info.pending.unwrap();
        assert_eq!(pending.len(), 2);
        assert_eq!(
            pending[0].labels.get("repo"),
            Some(&String::from("windsource/woodpecker-test"))
        );
        assert_eq!(wp_queue_info.stats.worker_count, 0);
    }

    #[test]
    fn parse_queue_info_pending_running() {
        static QUEUE_INFO: &str = r#"{
  "pending": [
    {
      "id": "12",
      "data": "REDACTED",
      "labels": {
        "platform": "",
        "repo": "windsource/woodpecker-test"
      },
      "dependencies": null,
      "run_on": null,
      "dep_status": {},
      "agent_id": 0
    }
  ],
  "waiting_on_deps": null,
  "running": [
    {
      "id": "11",
      "data": "REDACTED",
      "labels": {
        "platform": "",
        "repo": "windsource/woodpecker-test"
      },
      "dependencies": null,
      "run_on": null,
      "dep_status": {},
      "agent_id": 1
    }
  ],
  "stats": {
    "worker_count": 0,
    "pending_count": 1,
    "waiting_on_deps_count": 0,
    "running_count": 1,
    "completed_count": 0
  },
  "paused": false
}"#;
        let wp_queue_info: WpQueueInfo = serde_json::from_str(QUEUE_INFO).unwrap();
        let pending = wp_queue_info.pending.unwrap();
        let running = wp_queue_info.running.unwrap();
        assert_eq!(pending.len(), 1);
        assert_eq!(running.len(), 1);
        assert_eq!(
            running[0].labels.get("repo"),
            Some(&String::from("windsource/woodpecker-test"))
        );
        assert_eq!(wp_queue_info.stats.worker_count, 0);
    }

    #[tokio::test]
    async fn stop_running_agent_present_on_startup() {
        let mut mock = MockAgentProvider::new();
        mock.expect_stop().times(1).returning(|| Ok(()));
        mock.expect_is_running().times(1).returning(|| Ok(true));

        let mut strategy = Strategy::new(Box::new(mock), Duration::new(60, 0), "1".to_string());

        let queue_info = WpQueueInfo {
            pending: None,
            running: None,
            stats: WpQueueInfoStats {
                worker_count: 1,
                pending_count: 0,
                waiting_on_deps_count: 0,
                running_count: 0,
                completed_count: 0,
            },
        };
        strategy.apply(&queue_info).await;
    }

    #[tokio::test]
    async fn start_and_stop_agent() {
        let mut mock = MockAgentProvider::new();
        mock.expect_start().times(1).returning(|| Ok(()));
        mock.expect_stop().times(1).returning(|| Ok(()));
        let mut running_count = 0;
        mock.expect_is_running().times(2).returning(move || {
            if running_count == 0 {
                running_count += 1;
                Ok(false)
            } else {
                Ok(true)
            }
        });
        mock.expect_supports_labels().times(1).returning(|_| true);

        let d = Duration::new(5, 0);

        let mut strategy = Strategy::new(Box::new(mock), d, "1".to_string());

        let queue_info = WpQueueInfo {
            pending: Some(vec![WpQueueInfoPending {
                id: "0".to_string(),
                labels: HashMap::new(),
            }]),
            running: None,
            stats: WpQueueInfoStats {
                worker_count: 0,
                pending_count: 1,
                waiting_on_deps_count: 0,
                running_count: 0,
                completed_count: 0,
            },
        };
        strategy.apply(&queue_info).await;

        sleep(d).await;

        let queue_info = WpQueueInfo {
            pending: None,
            running: None,
            stats: WpQueueInfoStats {
                worker_count: 1,
                pending_count: 0,
                waiting_on_deps_count: 0,
                running_count: 0,
                completed_count: 0,
            },
        };
        strategy.apply(&queue_info).await;
    }

    #[tokio::test]
    async fn start_and_not_stop_agent() {
        let mut mock = MockAgentProvider::new();
        mock.expect_start().times(1).returning(|| Ok(()));
        mock.expect_stop().times(0).returning(|| Ok(()));
        let mut running_count = 0;
        mock.expect_is_running().times(2).returning(move || {
            if running_count == 0 {
                running_count += 1;
                Ok(false)
            } else {
                Ok(true)
            }
        });
        mock.expect_supports_labels().times(1).returning(|_| true);

        let d = Duration::new(5, 0);

        let mut strategy = Strategy::new(Box::new(mock), d, "1".to_string());

        let queue_info = WpQueueInfo {
            pending: Some(vec![WpQueueInfoPending {
                id: "0".to_string(),
                labels: HashMap::new(),
            }]),
            running: None,
            stats: WpQueueInfoStats {
                worker_count: 0,
                pending_count: 1,
                waiting_on_deps_count: 0,
                running_count: 0,
                completed_count: 0,
            },
        };
        strategy.apply(&queue_info).await;

        let queue_info = WpQueueInfo {
            pending: None,
            running: None,
            stats: WpQueueInfoStats {
                worker_count: 1,
                pending_count: 0,
                waiting_on_deps_count: 0,
                running_count: 0,
                completed_count: 0,
            },
        };
        strategy.apply(&queue_info).await;
    }
}
