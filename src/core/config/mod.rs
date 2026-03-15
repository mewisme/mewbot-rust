use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub discord_token: String,
    pub command_prefix: String,
    pub admin_user_id: Option<u64>,
}

impl Config {
    pub fn load() -> Result<Self> {
        dotenv::dotenv().ok();

        let discord_token =
            env::var("DISCORD_TOKEN").context("DISCORD_TOKEN not found in environment")?;

        let command_prefix = env::var("COMMAND_PREFIX").unwrap_or_else(|_| "m/".to_string());

        let admin_user_id = env::var("ADMIN_USER_ID")
            .ok()
            .and_then(|s| s.parse::<u64>().ok());

        Ok(Config {
            discord_token,
            command_prefix,
            admin_user_id,
        })
    }

    pub fn is_admin(&self, user_id: u64) -> bool {
        self.admin_user_id.map_or(false, |id| id == user_id)
    }
}
