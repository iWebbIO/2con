use crate::model::{ProfileItem, AppSettings, RoutingRule};
use std::error::Error;

pub trait CoreEngine: Send + Sync {
    fn generate_config(
        &self,
        profile: &ProfileItem,
        settings: &AppSettings,
        routing_rules: &[RoutingRule],
    ) -> Result<String, Box<dyn Error + Send + Sync>>;

    fn get_stats_command(&self, server: &str) -> Option<Vec<String>>;
}

pub mod config_builder;
pub mod xray;
pub mod singbox;
pub mod process;
