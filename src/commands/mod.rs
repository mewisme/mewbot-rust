use async_trait::async_trait;
use serenity::builder::CreateCommand;
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::Context;
use std::time::Duration;

#[async_trait]
pub trait Command: Send + Sync {
    fn name(&self) -> &'static str;
    fn description(&self) -> &'static str;

    fn register_slash(&self, cmd: &mut CreateCommand);
    async fn run_slash(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> anyhow::Result<()>;

    fn prefix(&self) -> Option<&'static str>;
    async fn run_prefix(&self, ctx: &Context, msg: &Message, args: &[&str]) -> anyhow::Result<()>;

    fn aliases(&self) -> &'static [&'static str] {
        &[]
    }

    fn cooldown_duration(&self) -> Duration {
        Duration::from_secs(3)
    }
}

pub mod flashback;
pub mod help;
