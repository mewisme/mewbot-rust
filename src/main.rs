mod cli;
mod commands;
mod config;
mod context;
mod events;
mod permissions;
mod registry;
mod utils;
mod wallet_store;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use config::Config;
use context::BotContext;
use registry::Registry;
use serenity::model::gateway::GatewayIntents;
use serenity::prelude::*;
use std::sync::Arc;

struct Handler {
    bot_context: Arc<BotContext>,
}

#[async_trait::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: serenity::model::gateway::Ready) {
        events::ready::ready(ctx, ready, &self.bot_context).await;
    }

    async fn message(&self, ctx: Context, msg: serenity::model::channel::Message) {
        events::message::message(ctx, msg, &self.bot_context).await;
    }

    async fn interaction_create(
        &self,
        ctx: Context,
        interaction: serenity::model::application::Interaction,
    ) {
        events::interaction::interaction(ctx, interaction, &self.bot_context).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            cli::Commands::Generate { name } => {
                cli::generate_command(&name)?;
                return Ok(());
            }
            cli::Commands::Version => {
                cli::show_version();
                return Ok(());
            }
            cli::Commands::Config { key } => {
                cli::show_config(&key)?;
                return Ok(());
            }
        }
    }

    let config = Config::load()?;
    crate::done!("Configuration loaded successfully");

    let registry = Registry::new();

    let bot_context = Arc::new(BotContext::new(config.clone(), registry));

    {
        let mut reg = bot_context.registry.lock().await;
        utils::register_commands(&mut reg, bot_context.clone());
    }
    crate::done!("Commands registered");

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS;

    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(Handler {
            bot_context: bot_context.clone(),
        })
        .await?;

    crate::info!("Starting bot...");
    client.start().await?;

    Ok(())
}
