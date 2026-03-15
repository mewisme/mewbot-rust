use crate::core::permissions::PermissionLevel;
use async_trait::async_trait;
use serenity::builder::CreateCommand;
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::Context;
use std::time::Duration;

#[derive(Clone, Copy, Debug)]
pub struct SubCommandInfo {
    pub name: &'static str,
    pub description: &'static str,
    pub aliases: &'static [&'static str],
}

#[derive(Clone, Debug)]
pub struct CommandInfo {
    pub name: String,
    pub description: String,
    pub prefix: Option<String>,
    pub aliases: Vec<String>,
    pub cooldown_secs: u64,
    pub required_permission: Option<PermissionLevel>,
    pub version: String,
    pub subcommands: Vec<SubCommandInfo>,
}

pub trait CommandLister: Send + Sync {
    fn list(&self) -> Vec<CommandInfo>;
}

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

    fn required_permission_level(&self) -> Option<PermissionLevel> {
        None
    }

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn subcommands(&self) -> &'static [SubCommandInfo] {
        &[]
    }

    fn resolve_subcommand(&self, name_or_alias: &str) -> Option<&'static str> {
        self.subcommands().iter().find(|s| {
            s.name.eq_ignore_ascii_case(name_or_alias)
                || s.aliases.iter().any(|a| a.eq_ignore_ascii_case(name_or_alias))
        }).map(|s| s.name)
    }
}
