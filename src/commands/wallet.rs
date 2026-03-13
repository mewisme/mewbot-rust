//! Wallet command: check (default), credit, debit, init, reset.
//! Data stored in `data/wallet.json`. Permission: check self = anyone; check others / add / remove = owner or admin.

use crate::commands::Command;
use crate::permissions::{get_permission_level, has_permission, PermissionLevel};
use crate::wallet_store::{load_wallet, save_wallet, WALLET_LOCK};
use async_trait::async_trait;
use serenity::all::{CommandDataOptionValue, CommandOptionType};
use serenity::builder::{
    CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseMessage, CreateMessage,
};
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::model::id::UserId;
use serenity::model::permissions::Permissions;
use serenity::prelude::Context;
use std::sync::Arc;
use std::time::Duration;

pub struct Wallet;

fn now_iso() -> String {
    chrono::Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

fn is_human_user_in_resolved(interaction: &CommandInteraction, user_id: UserId) -> Option<bool> {
    interaction
        .data
        .resolved
        .users
        .get(&user_id)
        .map(|u| !u.bot)
}

fn filter_human_mentions(msg: &Message) -> Vec<UserId> {
    msg.mentions
        .iter()
        .filter(|u| !u.bot)
        .map(|u| u.id)
        .collect()
}

#[async_trait]
impl Command for Wallet {
    fn name(&self) -> &'static str {
        "wallet"
    }

    fn description(&self) -> &'static str {
        "Check or manage wallet balance (check / credit / debit / init / reset)"
    }

    fn register_slash(&self, cmd: &mut CreateCommand) {
        *cmd = CreateCommand::new("wallet")
            .description("Check or manage wallet balance")
            .add_option(
                CreateCommandOption::new(serenity::all::CommandOptionType::SubCommand, "check", "Check wallet balance (self or mentioned users)")
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::User, "user", "User to check (optional, default: self)")
                            .required(false),
                    ),
            )
            .add_option(
                CreateCommandOption::new(serenity::all::CommandOptionType::SubCommand, "credit", "Add money (owner/admin only)")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Integer, "amount", "Amount to add").required(true).min_int_value(1))
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::User, "user", "User to credit (optional, default: self)")
                            .required(false),
                    ),
            )
            .add_option(
                CreateCommandOption::new(serenity::all::CommandOptionType::SubCommand, "debit", "Remove money (owner/admin only)")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Integer, "amount", "Amount to remove").required(true).min_int_value(1))
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::User, "user", "User to debit (optional, default: self)")
                            .required(false),
                    ),
            )
            .add_option(
                CreateCommandOption::new(serenity::all::CommandOptionType::SubCommand, "init", "Initialize wallet(s) (owner/admin only)")
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::User, "user", "User to init (optional; omit to init all server members, excluding bots)")
                            .required(false),
                    )
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::Integer, "amount", "Initial balance (optional, default: 0)")
                            .required(false)
                            .min_int_value(0),
                    ),
            )
            .add_option(
                CreateCommandOption::new(serenity::all::CommandOptionType::SubCommand, "reset", "Reset wallet(s) to balance (owner/admin only)")
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::User, "user", "User to reset (optional; omit to reset all server members, excluding bots)")
                            .required(false),
                    )
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::Integer, "amount", "Balance after reset (optional, default: 0)")
                            .required(false)
                            .min_int_value(0),
                    ),
            );
    }

    async fn run_slash(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> anyhow::Result<()> {
        let guild_id = match interaction.guild_id {
            Some(id) => id,
            None => {
                respond_ephemeral(interaction, &ctx, "This command can only be used in a server.").await?;
                return Ok(());
            }
        };

        let permission_data = match ctx.cache.guild(guild_id) {
            Some(guild) => {
                let owner_id = guild.owner_id;
                let channel_id = interaction.channel_id;
                let has_admin = interaction
                    .member
                    .as_ref()
                    .and_then(|m| {
                        guild
                            .channels
                            .get(&channel_id)
                            .map(|ch| guild.user_permissions_in(ch, m).contains(Permissions::ADMINISTRATOR))
                    })
                    .unwrap_or(false);
                Some((owner_id, has_admin))
            }
            None => None,
        };
        let (owner_id, has_administrator) = match permission_data {
            Some((o, h)) => (o, h),
            None => {
                respond_ephemeral(interaction, &ctx, "Could not load server information.").await?;
                return Ok(());
            }
        };
        let caller_level = get_permission_level(owner_id, interaction.user.id, has_administrator);

        let sub = interaction
            .data
            .options
            .first()
            .map(|o| (o.name.as_str(), &o.value));
        let (sub_name, nested) = match sub {
            Some((name, CommandDataOptionValue::SubCommand(opts))) => (name, opts.as_slice()),
            Some((name, _)) => (name, &[][..]),
            None => ("check", &[][..]),
        };

        match sub_name {
            "check" => {
                let target_ids: Vec<UserId> = nested
                    .iter()
                    .find(|o| o.name == "user")
                    .and_then(|o| o.value.as_user_id())
                    .map(|u| vec![u])
                    .unwrap_or_else(|| vec![interaction.user.id]);
                let target_ids: Vec<UserId> = if target_ids.is_empty() {
                    vec![interaction.user.id]
                } else {
                    target_ids
                };
                if target_ids.len() == 1 && target_ids[0] != interaction.user.id {
                    if let Some(is_human) = is_human_user_in_resolved(interaction, target_ids[0]) {
                        if !is_human {
                            respond_ephemeral(
                                interaction,
                                &ctx,
                                "Bots and application accounts are not supported.",
                            )
                            .await?;
                            return Ok(());
                        }
                    }
                }
                if target_ids.len() > 1 || (target_ids.len() == 1 && target_ids[0] != interaction.user.id) {
                    if !has_permission(caller_level, PermissionLevel::Admin) {
                        respond_ephemeral(interaction, &ctx, "You need **server admin or owner** to view others' wallets.").await?;
                        return Ok(());
                    }
                }
                let _guard = WALLET_LOCK.lock().await;
                let data = load_wallet().await;
                let mut not_inited = Vec::new();
                for uid in &target_ids {
                    if !data.has_user(uid.get()) {
                        let name = if *uid == interaction.user.id {
                            interaction.user.name.clone()
                        } else {
                            interaction
                                .data
                                .resolved
                                .users
                                .get(uid)
                                .map(|u| u.name.clone())
                                .unwrap_or_else(|| uid.get().to_string())
                        };
                        not_inited.push(name);
                    }
                }
                if !not_inited.is_empty() {
                    respond_ephemeral(
                        interaction,
                        &ctx,
                        &format!(
                            "The following user(s) have not been initialized: **{}**. Use **wallet init** first.",
                            not_inited.join(", ")
                        ),
                    )
                    .await?;
                    return Ok(());
                }
                let mut lines = Vec::new();
                for uid in &target_ids {
                    let bal = data.get_balance_if_exists(uid.get()).unwrap_or(0);
                    let name = if *uid == interaction.user.id {
                        interaction.user.name.as_str()
                    } else {
                        interaction.data.resolved.users.get(uid).map(|u| u.name.as_str()).unwrap_or("Unknown")
                    };
                    lines.push(format!("**{}**: {}", name, bal));
                }
                let desc = lines.join("\n");
                interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .embed(CreateEmbed::new().title("Wallet").description(desc).color(0x00ff00)),
                        ),
                    )
                    .await?;
            }
            "init" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    respond_ephemeral(interaction, &ctx, "You need **server admin or owner** to init wallets.").await?;
                    return Ok(());
                }
                let target_ids: Vec<UserId> = nested
                    .iter()
                    .find(|o| o.name == "user")
                    .and_then(|o| o.value.as_user_id())
                    .map(|u| vec![u])
                    .unwrap_or_else(|| Vec::new());
                let target_ids: Vec<UserId> = if target_ids.is_empty() {
                    let from_cache: Vec<UserId> = match ctx.cache.guild(guild_id) {
                        Some(g) => g
                            .members
                            .values()
                            .filter(|m| !m.user.bot)
                            .map(|m| m.user.id)
                            .collect(),
                        None => vec![],
                    };
                    if !from_cache.is_empty() {
                        from_cache
                    } else {
                        match guild_id.members(&ctx.http, Some(1000), None).await {
                            Ok(members) => members
                                .into_iter()
                                .filter(|m| !m.user.bot)
                                .map(|m| m.user.id)
                                .collect(),
                            Err(_) => vec![],
                        }
                    }
                } else {
                    target_ids
                };
                if target_ids.len() == 1 {
                    if let Some(is_human) = is_human_user_in_resolved(interaction, target_ids[0]) {
                        if !is_human {
                            respond_ephemeral(
                                interaction,
                                &ctx,
                                "Bots and application accounts are not supported.",
                            )
                            .await?;
                            return Ok(());
                        }
                    }
                }
                if target_ids.is_empty() {
                    respond_ephemeral(interaction, &ctx, "No users to init (could not load server members from cache or API).").await?;
                    return Ok(());
                }
                let init_balance: i64 = nested
                    .iter()
                    .find(|o| o.name == "amount")
                    .and_then(|o| o.value.as_i64())
                    .unwrap_or(0)
                    .max(0);
                let _guard = WALLET_LOCK.lock().await;
                let mut data = load_wallet().await;
                let now = now_iso();
                let mut created = 0usize;
                for uid in &target_ids {
                    if data.init_user_if_new(uid.get(), init_balance, &now) {
                        created += 1;
                    }
                }
                save_wallet(&data).await?;
                let skipped = target_ids.len().saturating_sub(created);
                let msg_text = if skipped == 0 {
                    format!("Initialized **{}** user(s) with balance **{}**.", created, init_balance)
                } else {
                    format!(
                        "Initialized **{}** new user(s) with balance **{}**. Skipped **{}** already in wallet.",
                        created, init_balance, skipped
                    )
                };
                respond_embed(interaction, &ctx, "Wallet Init", &msg_text, 0x00ff00).await?;
            }
            "reset" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    respond_ephemeral(interaction, &ctx, "You need **server admin or owner** to reset wallets.").await?;
                    return Ok(());
                }
                let target_ids: Vec<UserId> = nested
                    .iter()
                    .find(|o| o.name == "user")
                    .and_then(|o| o.value.as_user_id())
                    .map(|u| vec![u])
                    .unwrap_or_else(|| Vec::new());
                let target_ids: Vec<UserId> = if target_ids.is_empty() {
                    let from_cache: Vec<UserId> = match ctx.cache.guild(guild_id) {
                        Some(g) => g
                            .members
                            .values()
                            .filter(|m| !m.user.bot)
                            .map(|m| m.user.id)
                            .collect(),
                        None => vec![],
                    };
                    if !from_cache.is_empty() {
                        from_cache
                    } else {
                        match guild_id.members(&ctx.http, Some(1000), None).await {
                            Ok(members) => members
                                .into_iter()
                                .filter(|m| !m.user.bot)
                                .map(|m| m.user.id)
                                .collect(),
                            Err(_) => vec![],
                        }
                    }
                } else {
                    target_ids
                };
                if target_ids.len() == 1 {
                    if let Some(is_human) = is_human_user_in_resolved(interaction, target_ids[0]) {
                        if !is_human {
                            respond_ephemeral(
                                interaction,
                                &ctx,
                                "Bots and application accounts are not supported.",
                            )
                            .await?;
                            return Ok(());
                        }
                    }
                }
                if target_ids.is_empty() {
                    respond_ephemeral(interaction, &ctx, "No users to reset (could not load server members from cache or API).").await?;
                    return Ok(());
                }
                let reset_balance: i64 = nested
                    .iter()
                    .find(|o| o.name == "amount")
                    .and_then(|o| o.value.as_i64())
                    .unwrap_or(0)
                    .max(0);
                let _guard = WALLET_LOCK.lock().await;
                let mut data = load_wallet().await;
                let now = now_iso();
                for uid in &target_ids {
                    data.init_user(uid.get(), reset_balance, &now);
                }
                save_wallet(&data).await?;
                let count = target_ids.len();
                respond_embed(
                    interaction,
                    &ctx,
                    "Wallet Reset",
                    &format!("Reset **{}** user(s) to balance **{}**.", count, reset_balance),
                    0x00ff00,
                )
                .await?;
            }
            "credit" | "debit" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    respond_ephemeral(interaction, &ctx, "You need **server admin or owner** to credit/debit balance.").await?;
                    return Ok(());
                }
                let amount = nested
                    .iter()
                    .find(|o| o.name == "amount")
                    .and_then(|o| o.value.as_i64())
                    .unwrap_or(0);
                if amount <= 0 {
                    respond_ephemeral(interaction, &ctx, "Invalid amount (must be positive).").await?;
                    return Ok(());
                }
                let target_ids: Vec<UserId> = nested
                    .iter()
                    .find(|o| o.name == "user")
                    .and_then(|o| o.value.as_user_id())
                    .map(|u| vec![u])
                    .unwrap_or_else(|| vec![interaction.user.id]);
                let target_ids: Vec<UserId> = if target_ids.is_empty() {
                    vec![interaction.user.id]
                } else {
                    target_ids
                };
                if target_ids.len() == 1 && target_ids[0] != interaction.user.id {
                    if let Some(is_human) = is_human_user_in_resolved(interaction, target_ids[0]) {
                        if !is_human {
                            respond_ephemeral(
                                interaction,
                                &ctx,
                                "Bots and application accounts are not supported.",
                            )
                            .await?;
                            return Ok(());
                        }
                    }
                }
                let _guard = WALLET_LOCK.lock().await;
                let mut data = load_wallet().await;
                let mut not_inited = Vec::new();
                for uid in &target_ids {
                    if !data.has_user(uid.get()) {
                        let name = if *uid == interaction.user.id {
                            interaction.user.name.clone()
                        } else {
                            interaction
                                .data
                                .resolved
                                .users
                                .get(uid)
                                .map(|u| u.name.clone())
                                .unwrap_or_else(|| uid.get().to_string())
                        };
                        not_inited.push(name);
                    }
                }
                if !not_inited.is_empty() {
                    respond_ephemeral(
                        interaction,
                        &ctx,
                        &format!(
                            "The following user(s) have not been initialized: **{}**. Use **wallet init** first.",
                            not_inited.join(", ")
                        ),
                    )
                    .await?;
                    return Ok(());
                }
                let now = now_iso();
                if sub_name == "credit" {
                    for uid in &target_ids {
                        data.add_balance(uid.get(), amount, &now);
                    }
                    let names: Vec<String> = target_ids
                        .iter()
                        .map(|uid| {
                            if *uid == interaction.user.id {
                                interaction.user.name.clone()
                            } else {
                                interaction
                                    .data
                                    .resolved
                                    .users
                                    .get(uid)
                                    .map(|u| u.name.clone())
                                    .unwrap_or_else(|| uid.get().to_string())
                            }
                        })
                        .collect();
                    save_wallet(&data).await?;
                    respond_embed(
                        interaction,
                        &ctx,
                        "Wallet",
                        &format!("Added **{}** to: {}", amount, names.join(", ")),
                        0x00ff00,
                    )
                    .await?;
                } else {
                    let mut ok_names = Vec::new();
                    let mut failed = Vec::new();
                    for uid in &target_ids {
                        let name = if *uid == interaction.user.id {
                            interaction.user.name.clone()
                        } else {
                            interaction
                                .data
                                .resolved
                                .users
                                .get(uid)
                                .map(|u| u.name.clone())
                                .unwrap_or_else(|| uid.get().to_string())
                        };
                        match data.subtract_balance(uid.get(), amount, &now) {
                            Ok(_) => ok_names.push(name),
                            Err(_) => failed.push(name),
                        }
                    }
                    save_wallet(&data).await?;
                    if failed.is_empty() {
                        respond_embed(
                            interaction,
                            &ctx,
                            "Wallet",
                            &format!("Removed **{}** from: {}", amount, ok_names.join(", ")),
                            0x00ff00,
                        )
                        .await?;
                    } else {
                        respond_embed(
                            interaction,
                            &ctx,
                            "Wallet (partial)",
                            &format!("Removed from: {}. Insufficient balance: {}", ok_names.join(", "), failed.join(", ")),
                            0xffaa00,
                        )
                        .await?;
                    }
                }
            }
            _ => {
                // default: check self
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    // viewing self only
                }
                let _guard = WALLET_LOCK.lock().await;
                let data = load_wallet().await;
                if !data.has_user(interaction.user.id.get()) {
                    respond_ephemeral(
                        interaction,
                        &ctx,
                        "You have not been initialized. Use **wallet init** first.",
                    )
                    .await?;
                    return Ok(());
                }
                let bal = data.get_balance_if_exists(interaction.user.id.get()).unwrap_or(0);
                respond_embed(
                    interaction,
                    &ctx,
                    "Wallet",
                    &format!("**{}**: {}", interaction.user.name, bal),
                    0x00ff00,
                )
                .await?;
            }
        }
        Ok(())
    }

    fn prefix(&self) -> Option<&'static str> {
        Some("wallet")
    }

    async fn run_prefix(
        &self,
        ctx: &Context,
        msg: &Message,
        args: &[&str],
    ) -> anyhow::Result<()> {
        let guild_id = match msg.guild_id {
            Some(id) => id,
            None => {
                crate::utils::send_error_message(msg, ctx, "This command can only be used in a server.").await;
                return Ok(());
            }
        };
        let permission_data = match ctx.cache.guild(guild_id) {
            Some(guild) => {
                let owner_id = guild.owner_id;
                let has_admin = guild
                    .members
                    .get(&msg.author.id)
                    .and_then(|m| {
                        guild
                            .channels
                            .get(&msg.channel_id)
                            .map(|ch| guild.user_permissions_in(ch, m).contains(Permissions::ADMINISTRATOR))
                    })
                    .unwrap_or(false)
                    || msg
                        .member
                        .as_ref()
                        .and_then(|pm| pm.permissions.as_ref())
                        .map_or(false, |p| p.contains(Permissions::ADMINISTRATOR));
                Some((owner_id, has_admin))
            }
            None => None,
        };
        let (owner_id, has_administrator) = match permission_data {
            Some((o, h)) => (o, h),
            None => {
                crate::utils::send_error_message(msg, ctx, "Could not load server information.").await;
                return Ok(());
            }
        };
        let caller_level = get_permission_level(owner_id, msg.author.id, has_administrator);

        let mut args_iter = args.iter().peekable();
        let sub: String = args_iter
            .next()
            .map(|s| (*s).to_lowercase())
            .unwrap_or_else(|| "check".to_string());

        let mentions: Vec<UserId> = filter_human_mentions(msg);

        match sub.as_str() {
            "check" => {
                let target_ids: Vec<UserId> = if mentions.is_empty() {
                    vec![msg.author.id]
                } else {
                    if !has_permission(caller_level, PermissionLevel::Admin) {
                        crate::utils::send_error_message(msg, ctx, "You need **server admin or owner** to view others' wallets.").await;
                        return Ok(());
                    }
                    mentions
                };
                let _guard = WALLET_LOCK.lock().await;
                let data = load_wallet().await;
                let mut not_inited = Vec::new();
                for uid in &target_ids {
                    if !data.has_user(uid.get()) {
                        let name = if *uid == msg.author.id {
                            msg.author.name.clone()
                        } else {
                            msg.mentions.iter().find(|u| u.id == *uid).map(|u| u.name.clone()).unwrap_or_else(|| uid.get().to_string())
                        };
                        not_inited.push(name);
                    }
                }
                if !not_inited.is_empty() {
                    crate::utils::send_error_message(
                        msg,
                        ctx,
                        &format!(
                            "The following user(s) have not been initialized: **{}**. Use **wallet init** first.",
                            not_inited.join(", ")
                        ),
                    )
                    .await;
                    return Ok(());
                }
                let mut lines = Vec::new();
                for uid in &target_ids {
                    let bal = data.get_balance_if_exists(uid.get()).unwrap_or(0);
                    let name = msg.mentions.iter().find(|u| u.id == *uid).map(|u| u.name.as_str()).unwrap_or_else(|| {
                        if *uid == msg.author.id {
                            msg.author.name.as_str()
                        } else {
                            "Unknown"
                        }
                    });
                    lines.push(format!("**{}**: {}", name, bal));
                }
                let desc = lines.join("\n");
                msg.channel_id
                    .send_message(&ctx.http, CreateMessage::new().embed(CreateEmbed::new().title("Wallet").description(desc).color(0x00ff00)))
                    .await?;
            }
            "init" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    crate::utils::send_error_message(msg, ctx, "You need **server admin or owner** to init wallets.").await;
                    return Ok(());
                }
                let target_ids: Vec<UserId> = if mentions.is_empty() {
                    let from_cache: Vec<UserId> = match ctx.cache.guild(guild_id) {
                        Some(g) => g
                            .members
                            .values()
                            .filter(|m| !m.user.bot)
                            .map(|m| m.user.id)
                            .collect(),
                        None => vec![],
                    };
                    if !from_cache.is_empty() {
                        from_cache
                    } else {
                        match guild_id.members(&ctx.http, Some(1000), None).await {
                            Ok(members) => members
                                .into_iter()
                                .filter(|m| !m.user.bot)
                                .map(|m| m.user.id)
                                .collect(),
                            Err(_) => vec![],
                        }
                    }
                } else {
                    mentions
                };
                if target_ids.is_empty() {
                    crate::utils::send_error_message(msg, ctx, "No users to init (could not load server members from cache or API).").await;
                    return Ok(());
                }
                let init_balance: i64 = args_iter
                    .filter_map(|s| s.parse::<i64>().ok())
                    .find(|&n| n >= 0)
                    .unwrap_or(0);
                let _guard = WALLET_LOCK.lock().await;
                let mut data = load_wallet().await;
                let now = now_iso();
                let mut created = 0usize;
                for uid in &target_ids {
                    if data.init_user_if_new(uid.get(), init_balance, &now) {
                        created += 1;
                    }
                }
                save_wallet(&data).await?;
                let skipped = target_ids.len().saturating_sub(created);
                let desc = if skipped == 0 {
                    format!("Initialized **{}** user(s) with balance **{}**.", created, init_balance)
                } else {
                    format!(
                        "Initialized **{}** new user(s) with balance **{}**. Skipped **{}** already in wallet.",
                        created, init_balance, skipped
                    )
                };
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().embed(
                            CreateEmbed::new()
                                .title("Wallet Init")
                                .description(desc)
                                .color(0x00ff00),
                        ),
                    )
                    .await?;
            }
            "reset" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    crate::utils::send_error_message(msg, ctx, "You need **server admin or owner** to reset wallets.").await;
                    return Ok(());
                }
                let target_ids: Vec<UserId> = if mentions.is_empty() {
                    let from_cache: Vec<UserId> = match ctx.cache.guild(guild_id) {
                        Some(g) => g
                            .members
                            .values()
                            .filter(|m| !m.user.bot)
                            .map(|m| m.user.id)
                            .collect(),
                        None => vec![],
                    };
                    if !from_cache.is_empty() {
                        from_cache
                    } else {
                        match guild_id.members(&ctx.http, Some(1000), None).await {
                            Ok(members) => members
                                .into_iter()
                                .filter(|m| !m.user.bot)
                                .map(|m| m.user.id)
                                .collect(),
                            Err(_) => vec![],
                        }
                    }
                } else {
                    mentions
                };
                if target_ids.is_empty() {
                    crate::utils::send_error_message(msg, ctx, "No users to reset (could not load server members from cache or API).").await;
                    return Ok(());
                }
                let reset_balance: i64 = args_iter
                    .filter_map(|s| s.parse::<i64>().ok())
                    .find(|&n| n >= 0)
                    .unwrap_or(0);
                let _guard = WALLET_LOCK.lock().await;
                let mut data = load_wallet().await;
                let now = now_iso();
                for uid in &target_ids {
                    data.init_user(uid.get(), reset_balance, &now);
                }
                save_wallet(&data).await?;
                let count = target_ids.len();
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().embed(
                            CreateEmbed::new()
                                .title("Wallet Reset")
                                .description(format!("Reset **{}** user(s) to balance **{}**.", count, reset_balance))
                                .color(0x00ff00),
                        ),
                    )
                    .await?;
            }
            "credit" | "add" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    crate::utils::send_error_message(msg, ctx, "You need **server admin or owner** to add balance.").await;
                    return Ok(());
                }
                let amount_str = args_iter.next().map_or("0", |v| *v);
                let amount: i64 = amount_str.parse().unwrap_or(0);
                if amount <= 0 {
                    crate::utils::send_error_message(msg, ctx, "Invalid amount (positive number required).").await;
                    return Ok(());
                }
                let target_ids: Vec<UserId> = if mentions.is_empty() {
                    vec![msg.author.id]
                } else {
                    mentions
                };
                let _guard = WALLET_LOCK.lock().await;
                let mut data = load_wallet().await;
                let mut not_inited = Vec::new();
                for uid in &target_ids {
                    if !data.has_user(uid.get()) {
                        let name = if *uid == msg.author.id {
                            msg.author.name.clone()
                        } else {
                            msg.mentions.iter().find(|u| u.id == *uid).map(|u| u.name.clone()).unwrap_or_else(|| uid.get().to_string())
                        };
                        not_inited.push(name);
                    }
                }
                if !not_inited.is_empty() {
                    crate::utils::send_error_message(
                        msg,
                        ctx,
                        &format!(
                            "The following user(s) have not been initialized: **{}**. Use **wallet init** first.",
                            not_inited.join(", ")
                        ),
                    )
                    .await;
                    return Ok(());
                }
                let now = now_iso();
                for uid in &target_ids {
                    data.add_balance(uid.get(), amount, &now);
                }
                save_wallet(&data).await?;
                let names: Vec<String> = target_ids
                    .iter()
                    .map(|uid| {
                        if *uid == msg.author.id {
                            msg.author.name.clone()
                        } else {
                            msg.mentions.iter().find(|u| u.id == *uid).map(|u| u.name.clone()).unwrap_or_else(|| uid.get().to_string())
                        }
                    })
                    .collect();
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().embed(
                            CreateEmbed::new()
                                .title("Wallet")
                                .description(format!("Added **{}** to: {}", amount, names.join(", ")))
                                .color(0x00ff00),
                        ),
                    )
                    .await?;
            }
            "debit" | "remove" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    crate::utils::send_error_message(msg, ctx, "You need **server admin or owner** to remove balance.").await;
                    return Ok(());
                }
                let amount_str = args_iter.next().map_or("0", |v| *v);
                let amount: i64 = amount_str.parse().unwrap_or(0);
                if amount <= 0 {
                    crate::utils::send_error_message(msg, ctx, "Invalid amount (positive number required).").await;
                    return Ok(());
                }
                let target_ids: Vec<UserId> = if mentions.is_empty() {
                    vec![msg.author.id]
                } else {
                    mentions
                };
                let _guard = WALLET_LOCK.lock().await;
                let mut data = load_wallet().await;
                let mut not_inited = Vec::new();
                for uid in &target_ids {
                    if !data.has_user(uid.get()) {
                        let name = if *uid == msg.author.id {
                            msg.author.name.clone()
                        } else {
                            msg.mentions.iter().find(|u| u.id == *uid).map(|u| u.name.clone()).unwrap_or_else(|| uid.get().to_string())
                        };
                        not_inited.push(name);
                    }
                }
                if !not_inited.is_empty() {
                    crate::utils::send_error_message(
                        msg,
                        ctx,
                        &format!(
                            "The following user(s) have not been initialized: **{}**. Use **wallet init** first.",
                            not_inited.join(", ")
                        ),
                    )
                    .await;
                    return Ok(());
                }
                let now = now_iso();
                let mut ok_names = Vec::new();
                let mut failed = Vec::new();
                for uid in &target_ids {
                    let name = if *uid == msg.author.id {
                        msg.author.name.clone()
                    } else {
                        msg.mentions.iter().find(|u| u.id == *uid).map(|u| u.name.clone()).unwrap_or_else(|| uid.get().to_string())
                    };
                    match data.subtract_balance(uid.get(), amount, &now) {
                        Ok(_) => ok_names.push(name),
                        Err(_) => failed.push(name),
                    }
                }
                save_wallet(&data).await?;
                let desc = if failed.is_empty() {
                    format!("Removed **{}** from: {}", amount, ok_names.join(", "))
                } else {
                    format!("Removed from: {}. Insufficient balance: {}", ok_names.join(", "), failed.join(", "))
                };
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().embed(
                            CreateEmbed::new()
                                .title(if failed.is_empty() { "Wallet" } else { "Wallet (partial)" })
                                .description(desc)
                                .color(if failed.is_empty() { 0x00ff00 } else { 0xffaa00 }),
                        ),
                    )
                    .await?;
            }
            _ => {
                // no subcommand or unknown: treat as check self
                let _guard = WALLET_LOCK.lock().await;
                let data = load_wallet().await;
                if !data.has_user(msg.author.id.get()) {
                    crate::utils::send_error_message(msg, ctx, "You have not been initialized. Use **wallet init** first.").await;
                    return Ok(());
                }
                let bal = data.get_balance_if_exists(msg.author.id.get()).unwrap_or(0);
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new().embed(
                            CreateEmbed::new()
                                .title("Wallet")
                                .description(format!("**{}**: {}", msg.author.name, bal))
                                .color(0x00ff00),
                        ),
                    )
                    .await?;
            }
        }
        Ok(())
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["w", "bal", "balance"]
    }

    fn cooldown_duration(&self) -> Duration {
        Duration::from_secs(2)
    }

    fn required_permission_level(&self) -> Option<PermissionLevel> {
        None
    }
}

async fn respond_ephemeral(interaction: &CommandInteraction, ctx: &Context, text: &str) -> anyhow::Result<()> {
    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().content(text).ephemeral(true),
            ),
        )
        .await?;
    Ok(())
}

async fn respond_embed(
    interaction: &CommandInteraction,
    ctx: &Context,
    title: &str,
    description: &str,
    color: u32,
) -> anyhow::Result<()> {
    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .embed(CreateEmbed::new().title(title).description(description).color(color)),
            ),
        )
        .await?;
    Ok(())
}

pub fn create() -> Arc<dyn Command> {
    Arc::new(Wallet)
}
