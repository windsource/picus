use async_trait::async_trait;
#[cfg(test)]
use mockall::automock;
use std::{collections::HashMap, error::Error};

pub type Labels = HashMap<String, String>;

#[cfg_attr(test, automock)]
#[async_trait]
pub trait AgentProvider {
    async fn start(&self) -> Result<(), Box<dyn Error>>;
    async fn stop(&self) -> Result<(), Box<dyn Error>>;
    async fn is_running(&self) -> Result<bool, Box<dyn Error>>;
    fn supports_labels(&self, labels: &Labels) -> bool;
}

pub struct FilterLabels(Labels);

impl FilterLabels {
    pub fn from_string(filter_labels: &str) -> FilterLabels {
        let mut labels: Labels = HashMap::new();
        filter_labels.split(',').for_each(|s| {
            let kv: Vec<_> = s.split('=').collect();
            labels.insert(kv[0].to_string(), kv[1].to_string());
        });
        FilterLabels(labels)
    }
    pub fn supports(&self, labels: &Labels) -> bool {
        for (k, v) in labels.iter() {
            if let Some(filter_value) = self.0.get(k) {
                if filter_value != v && filter_value != "*" {
                    return false;
                }
            } else {
                return false;
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_labels_empty_labels() {
        let fl = FilterLabels::from_string("platform=linux/amd64,backend=docker");
        let labels: Labels = HashMap::new();
        assert!(fl.supports(&labels));
    }

    #[test]
    fn filter_labels_matching_label() {
        let fl = FilterLabels::from_string("platform=linux/amd64,backend=docker");
        let labels: Labels = HashMap::from([("platform".to_string(), "linux/amd64".to_string())]);
        assert!(fl.supports(&labels));
    }

    #[test]
    fn filter_labels_not_matching_label() {
        let fl = FilterLabels::from_string("platform=linux/amd64,backend=docker");
        let labels: Labels = HashMap::from([("platform".to_string(), "linux/arm64".to_string())]);
        assert!(!fl.supports(&labels));
    }
    #[test]
    fn filter_labels_wildcard() {
        let fl = FilterLabels::from_string("type=*,platform=linux/amd64,backend=docker");
        let labels: Labels = HashMap::from([("type".to_string(), "picus".to_string())]);
        assert!(fl.supports(&labels));
    }
}
