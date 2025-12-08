mod cli;
mod commands;
mod config;
mod context;
mod events;
mod registry;
mod utils;

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
    crate::utils::logger::init(config.enable_file_line_log);
    crate::done!("Configuration loaded successfully");

    let registry = Registry::new();

    let bot_context = Arc::new(BotContext::new(config.clone(), registry));

    {
        let mut reg = bot_context.registry.lock().await;
        utils::register_commands(&mut reg, bot_context.clone());
    }
    crate::done!("Commands registered");

    if config.dev_mode {
        crate::info!("Dev mode enabled - setting up auto-reload");
        let _bot_context_clone = bot_context.clone();
        tokio::spawn(async move {
            use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
            use std::path::Path;
            use std::sync::mpsc;
            use std::time::Duration;

            let (tx, rx) = mpsc::channel();
            let config = Config::default().with_poll_interval(Duration::from_secs(2));
            let mut watcher = RecommendedWatcher::new(tx, config).unwrap();

            let commands_dir = Path::new("src/commands");
            if commands_dir.exists() {
                watcher
                    .watch(commands_dir, RecursiveMode::NonRecursive)
                    .unwrap();
                crate::info!("Watching commands directory for changes...");
            }

            loop {
                match rx.recv() {
                    Ok(event) => {
                        crate::info!("File change detected: {:?}", event);
                        crate::warn!(
                            "Note: Command reload requires restart in current implementation"
                        );
                    }
                    Err(e) => {
                        crate::error!("Watch error: {:?}", e);
                    }
                }
            }
        });
    }

    let intents =
        GatewayIntents::GUILD_MESSAGES | GatewayIntents::MESSAGE_CONTENT | GatewayIntents::GUILDS;

    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(Handler {
            bot_context: bot_context.clone(),
        })
        .await?;

    crate::info!("Starting bot...");
    client.start().await?;

    Ok(())
}
