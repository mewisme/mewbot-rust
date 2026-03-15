use crate::core::config::Config;
use crate::core::registry::Registry;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

pub struct BotContext {
    pub config: Config,
    pub registry: Arc<Mutex<Registry>>,
    pub cooldowns: Arc<Mutex<HashMap<(u64, String), Instant>>>,
}

impl BotContext {
    pub fn new(config: Config, registry: Registry) -> Self {
        Self {
            config,
            registry: Arc::new(Mutex::new(registry)),
            cooldowns: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub async fn check_cooldown(&self, user_id: u64, command_name: &str) -> Option<Duration> {
        if self.config.is_admin(user_id) {
            return None;
        }

        let mut cooldowns = self.cooldowns.lock().await;
        let key = (user_id, command_name.to_string());

        if let Some(&last_used) = cooldowns.get(&key) {
            let elapsed = last_used.elapsed();

            if elapsed.as_secs() > 3600 {
                cooldowns.remove(&key);
                return None;
            }
            Some(elapsed)
        } else {
            None
        }
    }

    pub async fn set_cooldown(&self, user_id: u64, command_name: &str) {
        let mut cooldowns = self.cooldowns.lock().await;
        let key = (user_id, command_name.to_string());
        cooldowns.insert(key, Instant::now());
    }

    pub async fn get_cooldown_remaining(
        &self,
        user_id: u64,
        command_name: &str,
        cooldown_duration: Duration,
    ) -> Option<Duration> {
        if let Some(elapsed) = self.check_cooldown(user_id, command_name).await {
            if elapsed < cooldown_duration {
                Some(cooldown_duration - elapsed)
            } else {
                None
            }
        } else {
            None
        }
    }
}
