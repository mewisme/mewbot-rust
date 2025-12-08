use anyhow::{Context, Result};
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub discord_token: String,
    pub command_prefix: String,
    pub dev_mode: bool,
    pub admin_user_id: Option<u64>,
    pub enable_file_line_log: bool,
}

impl Config {
    pub fn load() -> Result<Self> {
        dotenv::dotenv().ok();

        let discord_token =
            env::var("DISCORD_TOKEN").context("DISCORD_TOKEN not found in environment")?;

        let command_prefix = env::var("COMMAND_PREFIX").unwrap_or_else(|_| "m/".to_string());

        let dev_mode = env::var("DEV_MODE")
            .unwrap_or_else(|_| "false".to_string())
            .parse::<bool>()
            .unwrap_or(false);

        let admin_user_id = env::var("ADMIN_USER_ID")
            .ok()
            .and_then(|s| s.parse::<u64>().ok());

        let enable_file_line_log = env::var("ENABLE_FILE_LINE_LOG")
            .unwrap_or_else(|_| "true".to_string())
            .parse::<bool>()
            .unwrap_or(true);

        Ok(Config {
            discord_token,
            command_prefix,
            dev_mode,
            admin_user_id,
            enable_file_line_log,
        })
    }

    pub fn is_admin(&self, user_id: u64) -> bool {
        self.admin_user_id.map_or(false, |id| id == user_id)
    }
}
