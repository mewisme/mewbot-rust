#![allow(dead_code, unused_imports)]

mod cli;
mod core;
mod plugins;

use anyhow::Result;
use clap::Parser;
use cli::Cli;
use core::{BotContext, Config, PluginApi, Registry, RegistryCommandLister};
use serenity::model::gateway::GatewayIntents;
use serenity::prelude::*;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

struct Handler {
    bot_context: Arc<BotContext>,
}

#[async_trait::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: serenity::model::gateway::Ready) {
        core::events::ready::ready(ctx, ready, &self.bot_context).await;
    }

    async fn message(&self, ctx: Context, msg: serenity::model::channel::Message) {
        core::events::message::message(ctx, msg, &self.bot_context).await;
    }

    async fn interaction_create(
        &self,
        ctx: Context,
        interaction: serenity::model::application::Interaction,
    ) {
        core::events::interaction::interaction(ctx, interaction, &self.bot_context).await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();
    dotenv::dotenv().ok();

    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            cli::Commands::GeneratePlugin { name } => {
                cli::generate_plugin(&name)?;
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

    let _ = io::stdout().flush();
    crate::info!("Bot version: v{}", core::updater::current_version());
    crate::info!("Author: {}", env!("CARGO_PKG_AUTHORS"));
    if std::env::var("MEWBOT_JUST_UPDATED").is_ok() {
        crate::done!("Updated and relaunched successfully 🎉");
    }

    if let Ok(release) = core::updater::fetch_latest().await {
        if let Some(asset) = core::updater::find_asset_for_current_platform(&release.files) {
            if core::updater::is_newer(&core::updater::current_version(), &release.version) {
                crate::info!("New version {} available, updating 🚀", release.version);
                if core::updater::run_update(&release, asset, async {})
                    .await
                    .is_ok()
                {
                    return Ok(());
                }
            } else {
                crate::done!("Bot is up to date 🎉");
            }
        }
    }

    let config = Config::load()?;
    crate::done!("Configuration loaded successfully 🔧");

    core::permissions::init_bot_owner_id(
        config.admin_user_id.map(serenity::model::id::UserId::new),
    );

    let registry = Registry::new();

    let bot_context = Arc::new(BotContext::new(config.clone(), registry));

    {
        let mut reg = bot_context.registry.lock().await;
        let command_lister = Arc::new(RegistryCommandLister(bot_context.registry.clone()));
        let api = PluginApi::new(bot_context.config.clone(), command_lister);
        plugins::register_commands(&mut reg, api);
    }
    crate::done!("Commands registered 📦");

    let config = &bot_context.config;
    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILDS
        | GatewayIntents::GUILD_MEMBERS;

    let mut client = Client::builder(&config.discord_token, intents)
        .event_handler(Handler {
            bot_context: bot_context.clone(),
        })
        .await?;

    let shard_manager = client.shard_manager.clone();
    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(60)).await;
            let release = match core::updater::fetch_latest().await {
                Ok(r) => r,
                Err(_) => continue,
            };
            let asset = match core::updater::find_asset_for_current_platform(&release.files) {
                Some(a) => a,
                None => continue,
            };
            if !core::updater::is_newer(&core::updater::current_version(), &release.version) {
                continue;
            }
            crate::info!("New version {} available, updating 🚀", release.version);
            if let Err(e) =
                core::updater::run_update(&release, asset, shard_manager.shutdown_all()).await
            {
                crate::error!("Update failed: {:?}", e);
                continue;
            }
            break;
        }
    });

    client.start().await?;

    Ok(())
}
