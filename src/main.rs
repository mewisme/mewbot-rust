mod cli;
mod commands;
mod config;
mod context;
mod events;
mod permissions;
mod registry;
mod updater;
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
    dotenv::dotenv().ok();

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

    let _ = io::stdout().flush();
    crate::info!("Bot version: v{}", updater::current_version());
    crate::info!("Author: {}", env!("CARGO_PKG_AUTHORS"));
    if std::env::var("MEWBOT_JUST_UPDATED").is_ok() {
        crate::done!("Updated and relaunched successfully");
    }

    let config = Config::load()?;
    crate::done!("Configuration loaded successfully");

    permissions::init_bot_owner_id(config.admin_user_id.map(serenity::model::id::UserId::new));

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

    let shard_manager = client.shard_manager.clone();
    if let Ok(release) = updater::fetch_latest().await {
        if let Some(asset) = updater::find_asset_for_current_platform(&release.files) {
            if updater::is_newer(&updater::current_version(), &release.version) {
                crate::info!("New version {} available, updating", release.version);
                if updater::run_update(&release, asset, shard_manager.shutdown_all())
                    .await
                    .is_ok()
                {
                    return Ok(());
                }
            } else {
                crate::done!("Bot is up to date !");
            }
        }
    }

    tokio::spawn(async move {
        loop {
            sleep(Duration::from_secs(60)).await;
            let release = match updater::fetch_latest().await {
                Ok(r) => r,
                Err(_) => continue,
            };
            let asset = match updater::find_asset_for_current_platform(&release.files) {
                Some(a) => a,
                None => continue,
            };
            if !updater::is_newer(&updater::current_version(), &release.version) {
                continue;
            }
            crate::info!("New version {} available, updating", release.version);
            if let Err(e) = updater::run_update(&release, asset, shard_manager.shutdown_all()).await
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
