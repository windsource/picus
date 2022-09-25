use crate::agent::AgentProvider;
use serde::Deserialize;
use std::time::Duration;
use tokio::time::Instant;

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
    // pending: u32,
    // waiting_on_deps: u32,
    // running: u32,
    stats: WpQueueInfoStats,
    #[serde(rename = "Paused")]
    paused: bool,
}

pub struct Strategy {
    agent_provider: Box<dyn AgentProvider>,
    last_time_running_agent: Option<Instant>,
    idle_time_before_stop: Duration,
}

impl Strategy {
    pub fn new(
        agent_provider: Box<dyn AgentProvider>,
        idle_time_before_stop: Duration,
    ) -> Strategy {
        Strategy {
            agent_provider,
            last_time_running_agent: None,
            idle_time_before_stop,
        }
    }

    pub async fn apply(&mut self, queue_info: &WpQueueInfo) {
        let stats = &queue_info.stats;
        if stats.worker_count == 0 && stats.running_count == 0 && stats.pending_count > 0 {
            println!("{} pending jobs. Starting agent.", stats.pending_count);
            let result = self.agent_provider.start().await;
            if let Err(err) = result {
                println!("AgentProvider could not start server: {}", err);
            }
            self.last_time_running_agent = Some(Instant::now());
        }
        if stats.running_count > 0 {
            self.last_time_running_agent = Some(Instant::now());
        }
        if stats.worker_count > 0 && stats.running_count == 0 && stats.pending_count == 0 {
            if let Some(last_time_running_agent) = self.last_time_running_agent {
                if last_time_running_agent.elapsed() > self.idle_time_before_stop {
                    println!("Idle timeout reached. Stopping agent.");
                    let _ = self.agent_provider.stop().await;
                }
            } else {
                println!("Still agent running from past. Stopping it.");
                let _ = self.agent_provider.stop().await;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tokio::time::sleep;
    use super::*;
    use crate::agent::MockAgentProvider;

    #[tokio::test]
    async fn stop_running_agent_present_on_startup() {
        let mut mock = MockAgentProvider::new();
        mock.expect_stop().times(1).returning(|| Ok(()));

        let mut strategy = Strategy::new(Box::new(mock), Duration::new(60, 0));

        let queue_info = WpQueueInfo {
            stats: WpQueueInfoStats {
                worker_count: 1,
                pending_count: 0,
                waiting_on_deps_count: 0,
                running_count: 0,
                completed_count: 0,
            },
            paused: false,
        };
        strategy.apply(&queue_info).await;
    }

    #[tokio::test]
    async fn start_and_stop_agent() {
        let mut mock = MockAgentProvider::new();
        mock.expect_start().times(1).returning(|| Ok(()));
        mock.expect_stop().times(1).returning(|| Ok(()));

        let d = Duration::new(5,0);

        let mut strategy = Strategy::new(Box::new(mock), d);

        let queue_info = WpQueueInfo {
            stats: WpQueueInfoStats {
                worker_count: 0,
                pending_count: 1,
                waiting_on_deps_count: 0,
                running_count: 0,
                completed_count: 0,
            },
            paused: false,
        };
        strategy.apply(&queue_info).await;
        
        sleep(d).await;

        let queue_info = WpQueueInfo {
            stats: WpQueueInfoStats {
                worker_count: 1,
                pending_count: 0,
                waiting_on_deps_count: 0,
                running_count: 0,
                completed_count: 0,
            },
            paused: false,
        };
        strategy.apply(&queue_info).await;
    }

    #[tokio::test]
    async fn start_and_not_stop_agent() {
        let mut mock = MockAgentProvider::new();
        mock.expect_start().times(1).returning(|| Ok(()));
        mock.expect_stop().times(0).returning(|| Ok(()));

        let d = Duration::new(5,0);

        let mut strategy = Strategy::new(Box::new(mock), d);

        let queue_info = WpQueueInfo {
            stats: WpQueueInfoStats {
                worker_count: 0,
                pending_count: 1,
                waiting_on_deps_count: 0,
                running_count: 0,
                completed_count: 0,
            },
            paused: false,
        };
        strategy.apply(&queue_info).await;
        
        let queue_info = WpQueueInfo {
            stats: WpQueueInfoStats {
                worker_count: 1,
                pending_count: 0,
                waiting_on_deps_count: 0,
                running_count: 0,
                completed_count: 0,
            },
            paused: false,
        };
        strategy.apply(&queue_info).await;
    }
}
