//! Wallet command: check (default), credit, debit, init, reset.
//! Data stored in `data/wallet.json`. Permission: check self = anyone; check others / add / remove = bot owner or server admin.

use crate::core::command::SubCommandInfo;
use crate::core::permissions::{get_permission_level, has_permission, PermissionLevel};
use crate::core::Command;
use crate::plugins::wallet::store::{load_wallet, save_wallet, WALLET_LOCK};
use async_trait::async_trait;
use serenity::all::{CommandDataOptionValue, CommandOptionType};
use serenity::builder::{
    CreateAllowedMentions, CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse,
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

fn user_mention(uid: UserId) -> String {
    format!("<@{}>", uid.get())
}

fn format_number_u64(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    let len = s.len();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result
}

fn format_balance_with_unit(bal: u64, unit: &str) -> String {
    let num = format_number_u64(bal);
    if unit.trim().is_empty() {
        num
    } else {
        format!("{} {}", num, unit.trim())
    }
}

#[async_trait]
impl Command for Wallet {
    fn name(&self) -> &'static str {
        "wallet"
    }

    fn description(&self) -> &'static str {
        "Check or manage wallet balance (check / add / remove / init / reset)"
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
                CreateCommandOption::new(serenity::all::CommandOptionType::SubCommand, "credit", "Add money (bot owner / server admin only)")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Integer, "amount", "Amount to add").required(true).min_int_value(1))
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::User, "user", "User to credit (optional, default: self)")
                            .required(false),
                    ),
            )
            .add_option(
                CreateCommandOption::new(serenity::all::CommandOptionType::SubCommand, "debit", "Remove money (bot owner / server admin only)")
                    .add_sub_option(CreateCommandOption::new(CommandOptionType::Integer, "amount", "Amount to remove").required(true).min_int_value(1))
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::User, "user", "User to debit (optional, default: self)")
                            .required(false),
                    ),
            )
            .add_option(
                CreateCommandOption::new(serenity::all::CommandOptionType::SubCommand, "init", "Initialize wallet(s) (bot owner / server admin only)")
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
                CreateCommandOption::new(serenity::all::CommandOptionType::SubCommand, "reset", "Reset wallet(s) to balance (bot owner / server admin only)")
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::User, "user", "User to reset (optional; omit to reset all server members, excluding bots)")
                            .required(false),
                    )
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::Integer, "amount", "Balance after reset (optional, default: 0)")
                            .required(false)
                            .min_int_value(0),
                    ),
            )
            .add_option(
                CreateCommandOption::new(serenity::all::CommandOptionType::SubCommand, "unit", "Set display unit for balance (e.g. xu) (bot owner / server admin only)")
                    .add_sub_option(
                        CreateCommandOption::new(CommandOptionType::String, "unit", "Unit name (e.g. xu, coins). Omit or empty to clear.")
                            .required(false),
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
                let guild_owner_id = guild.owner_id;
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
                Some((guild_owner_id, has_admin))
            }
            None => None,
        };
        let (guild_owner_id, has_administrator) = match permission_data {
            Some((o, h)) => (o, h),
            None => {
                respond_ephemeral(interaction, &ctx, "Could not load server information.").await?;
                return Ok(());
            }
        };
        let caller_level = get_permission_level(guild_owner_id, interaction.user.id, has_administrator);

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
                        respond_ephemeral(interaction, &ctx, "You need **bot owner** or **server admin** to view others' wallets.").await?;
                        return Ok(());
                    }
                }
                let _guard = WALLET_LOCK.lock().await;
                let data = load_wallet().await;
                let mut not_inited = Vec::new();
                for uid in &target_ids {
                    if !data.has_user(uid.get()) {
                        not_inited.push(user_mention(*uid));
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
                let unit = data.unit.as_str();
                let mut lines = Vec::new();
                for uid in &target_ids {
                    let bal = data.get_balance_if_exists(uid.get()).unwrap_or(0);
                    lines.push(format!("**{}**: {}", user_mention(*uid), format_balance_with_unit(bal, unit)));
                }
                let desc = lines.join("\n");
                interaction
                    .create_response(
                        &ctx.http,
                        CreateInteractionResponse::Message(
                            CreateInteractionResponseMessage::new()
                                .embed(CreateEmbed::new().title("Wallet").description(desc).color(0x00ff00))
                                .allowed_mentions(CreateAllowedMentions::new().empty_users()),
                        ),
                    )
                    .await?;
            }
            "init" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    respond_ephemeral(interaction, &ctx, "You need **bot owner** or **server admin** to init wallets.").await?;
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
                let bal_str = format_balance_with_unit(init_balance.max(0) as u64, &data.unit);
                let msg_text = if skipped == 0 {
                    format!("Initialized **{}** user(s) with balance **{}**.", created, bal_str)
                } else {
                    format!(
                        "Initialized **{}** new user(s) with balance **{}**. Skipped **{}** already in wallet.",
                        created, bal_str, skipped
                    )
                };
                respond_embed(interaction, &ctx, "Wallet Init", &msg_text, 0x00ff00).await?;
            }
            "reset" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    respond_ephemeral(interaction, &ctx, "You need **bot owner** or **server admin** to reset wallets.").await?;
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
                let bal_str = format_balance_with_unit(reset_balance.max(0) as u64, &data.unit);
                respond_embed(
                    interaction,
                    &ctx,
                    "Wallet Reset",
                    &format!("Reset **{}** user(s) to balance **{}**.", count, bal_str),
                    0x00ff00,
                )
                .await?;
            }
            "credit" | "debit" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    respond_ephemeral(interaction, &ctx, "You need **bot owner** or **server admin** to credit/debit balance.").await?;
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
                        not_inited.push(user_mention(*uid));
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
                        .map(|uid| user_mention(*uid))
                        .collect();
                    save_wallet(&data).await?;
                    let amount_str = format_balance_with_unit(amount as u64, &data.unit);
                    respond_embed(
                        interaction,
                        &ctx,
                        "Wallet",
                        &format!("Added **{}** to: {}", amount_str, names.join(", ")),
                        0x00ff00,
                    )
                    .await?;
                } else {
                    let mut ok_names = Vec::new();
                    let mut failed = Vec::new();
                    for uid in &target_ids {
                        let mention = user_mention(*uid);
                        match data.subtract_balance(uid.get(), amount, &now) {
                            Ok(_) => ok_names.push(mention),
                            Err(_) => failed.push(mention),
                        }
                    }
                    save_wallet(&data).await?;
                    let amount_str = format_balance_with_unit(amount as u64, &data.unit);
                    if failed.is_empty() {
                        respond_embed(
                            interaction,
                            &ctx,
                            "Wallet",
                            &format!("Removed **{}** from: {}", amount_str, ok_names.join(", ")),
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
            "unit" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    respond_ephemeral(interaction, &ctx, "You need **bot owner** or **server admin** to set the wallet unit.").await?;
                    return Ok(());
                }
                let unit_value = nested
                    .iter()
                    .find(|o| o.name == "unit")
                    .and_then(|o| o.value.as_str())
                    .unwrap_or("")
                    .trim()
                    .to_string();
                let _guard = WALLET_LOCK.lock().await;
                let mut data = load_wallet().await;
                data.unit = unit_value.clone();
                save_wallet(&data).await?;
                let msg = if unit_value.is_empty() {
                    "Wallet unit cleared (balance will show as number only).".to_string()
                } else {
                    format!("Wallet unit set to **{}**. Balances will display like: 1,000 {}", unit_value, unit_value)
                };
                respond_embed(interaction, &ctx, "Wallet Unit", &msg, 0x00ff00).await?;
            }
            _ => {
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
                let unit = data.unit.as_str();
                respond_embed(
                    interaction,
                    &ctx,
                    "Wallet",
                    &format!("**{}**: {}", user_mention(interaction.user.id), format_balance_with_unit(bal, unit)),
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
                crate::core::utils::send_error_message(msg, ctx, "This command can only be used in a server.").await;
                return Ok(());
            }
        };
        let permission_data = match ctx.cache.guild(guild_id) {
            Some(guild) => {
                let guild_owner_id = guild.owner_id;
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
                Some((guild_owner_id, has_admin))
            }
            None => None,
        };
        let (guild_owner_id, has_administrator) = match permission_data {
            Some((o, h)) => (o, h),
            None => {
                crate::core::utils::send_error_message(msg, ctx, "Could not load server information.").await;
                return Ok(());
            }
        };
        let caller_level = get_permission_level(guild_owner_id, msg.author.id, has_administrator);

        let mut args_iter = args.iter().peekable();
        let sub_arg: String = args_iter
            .next()
            .map(|s| (*s).to_lowercase())
            .unwrap_or_else(|| "check".to_string());
        let sub: &str = self.resolve_subcommand(&sub_arg).unwrap_or(&sub_arg);

        let mentions: Vec<UserId> = filter_human_mentions(msg);

        match sub {
            "check" => {
                let target_ids: Vec<UserId> = if mentions.is_empty() {
                    vec![msg.author.id]
                } else {
                    if !has_permission(caller_level, PermissionLevel::Admin) {
                        crate::core::utils::send_error_message(msg, ctx, "You need **bot owner** or **server admin** to view others' wallets.").await;
                        return Ok(());
                    }
                    mentions
                };
                let _guard = WALLET_LOCK.lock().await;
                let data = load_wallet().await;
                let mut not_inited = Vec::new();
                for uid in &target_ids {
                    if !data.has_user(uid.get()) {
                        not_inited.push(user_mention(*uid));
                    }
                }
                if !not_inited.is_empty() {
                    crate::core::utils::send_error_message(
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
                let unit = data.unit.as_str();
                let mut lines = Vec::new();
                for uid in &target_ids {
                    let bal = data.get_balance_if_exists(uid.get()).unwrap_or(0);
                    lines.push(format!("**{}**: {}", user_mention(*uid), format_balance_with_unit(bal, unit)));
                }
                let desc = lines.join("\n");
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new()
                            .embed(CreateEmbed::new().title("Wallet").description(desc).color(0x00ff00))
                            .allowed_mentions(CreateAllowedMentions::new().empty_users()),
                    )
                    .await?;
            }
            "init" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    crate::core::utils::send_error_message(msg, ctx, "You need **bot owner** or **server admin** to init wallets.").await;
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
                    crate::core::utils::send_error_message(msg, ctx, "No users to init (could not load server members from cache or API).").await;
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
                let bal_str = format_balance_with_unit(init_balance.max(0) as u64, &data.unit);
                let desc = if skipped == 0 {
                    format!("Initialized **{}** user(s) with balance **{}**.", created, bal_str)
                } else {
                    format!(
                        "Initialized **{}** new user(s) with balance **{}**. Skipped **{}** already in wallet.",
                        created, bal_str, skipped
                    )
                };
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new()
                        .embed(
                            CreateEmbed::new()
                                .title("Wallet Init")
                                .description(desc)
                                .color(0x00ff00),
                        )
                        .allowed_mentions(CreateAllowedMentions::new().empty_users()),
                    )
                    .await?;
            }
            "reset" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    crate::core::utils::send_error_message(msg, ctx, "You need **bot owner** or **server admin** to reset wallets.").await;
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
                    crate::core::utils::send_error_message(msg, ctx, "No users to reset (could not load server members from cache or API).").await;
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
                let bal_str = format_balance_with_unit(reset_balance.max(0) as u64, &data.unit);
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new()
                        .embed(
                            CreateEmbed::new()
                                .title("Wallet Reset")
                                .description(format!("Reset **{}** user(s) to balance **{}**.", count, bal_str))
                                .color(0x00ff00),
                        )
                        .allowed_mentions(CreateAllowedMentions::new().empty_users()),
                    )
                    .await?;
            }
            "credit" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    crate::core::utils::send_error_message(msg, ctx, "You need **bot owner** or **server admin** to add balance.").await;
                    return Ok(());
                }
                let amount_str = args_iter.next().map_or("0", |v| *v);
                let amount: i64 = amount_str.parse().unwrap_or(0);
                if amount <= 0 {
                    crate::core::utils::send_error_message(msg, ctx, "Invalid amount (positive number required).").await;
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
                        not_inited.push(user_mention(*uid));
                    }
                }
                if !not_inited.is_empty() {
                    crate::core::utils::send_error_message(
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
                let names: Vec<String> = target_ids.iter().map(|uid| user_mention(*uid)).collect();
                let amount_str = format_balance_with_unit(amount as u64, &data.unit);
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new()
                        .embed(
                            CreateEmbed::new()
                                .title("Wallet")
                                .description(format!("Added **{}** to: {}", amount_str, names.join(", ")))
                                .color(0x00ff00),
                        )
                        .allowed_mentions(CreateAllowedMentions::new().empty_users()),
                    )
                    .await?;
            }
            "debit" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    crate::core::utils::send_error_message(msg, ctx, "You need **bot owner** or **server admin** to remove balance.").await;
                    return Ok(());
                }
                let amount_str = args_iter.next().map_or("0", |v| *v);
                let amount: i64 = amount_str.parse().unwrap_or(0);
                if amount <= 0 {
                    crate::core::utils::send_error_message(msg, ctx, "Invalid amount (positive number required).").await;
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
                        not_inited.push(user_mention(*uid));
                    }
                }
                if !not_inited.is_empty() {
                    crate::core::utils::send_error_message(
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
                    let mention = user_mention(*uid);
                    match data.subtract_balance(uid.get(), amount, &now) {
                        Ok(_) => ok_names.push(mention),
                        Err(_) => failed.push(mention),
                    }
                }
                save_wallet(&data).await?;
                let amount_str = format_balance_with_unit(amount as u64, &data.unit);
                let desc = if failed.is_empty() {
                    format!("Removed **{}** from: {}", amount_str, ok_names.join(", "))
                } else {
                    format!("Removed from: {}. Insufficient balance: {}", ok_names.join(", "), failed.join(", "))
                };
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new()
                        .embed(
                            CreateEmbed::new()
                                .title(if failed.is_empty() { "Wallet" } else { "Wallet (partial)" })
                                .description(desc)
                                .color(if failed.is_empty() { 0x00ff00 } else { 0xffaa00 }),
                        )
                        .allowed_mentions(CreateAllowedMentions::new().empty_users()),
                    )
                    .await?;
            }
            "unit" => {
                if !has_permission(caller_level, PermissionLevel::Admin) {
                    crate::core::utils::send_error_message(msg, ctx, "You need **bot owner** or **server admin** to set the wallet unit.").await;
                    return Ok(());
                }
                let unit_value = args_iter
                    .next()
                    .map(|s| (*s).trim().to_string())
                    .unwrap_or_default();
                let _guard = WALLET_LOCK.lock().await;
                let mut data = load_wallet().await;
                data.unit = unit_value.clone();
                save_wallet(&data).await?;
                let desc = if unit_value.is_empty() {
                    "Wallet unit cleared (balance will show as number only).".to_string()
                } else {
                    format!("Wallet unit set to **{}**. Balances will display like: 1,000 {}", unit_value, unit_value)
                };
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new()
                            .embed(
                                CreateEmbed::new()
                                    .title("Wallet Unit")
                                    .description(desc)
                                    .color(0x00ff00),
                            )
                            .allowed_mentions(CreateAllowedMentions::new().empty_users()),
                    )
                    .await?;
            }
            _ => {
                let _guard = WALLET_LOCK.lock().await;
                let data = load_wallet().await;
                if !data.has_user(msg.author.id.get()) {
                    crate::core::utils::send_error_message(msg, ctx, "You have not been initialized. Use **wallet init** first.").await;
                    return Ok(());
                }
                let bal = data.get_balance_if_exists(msg.author.id.get()).unwrap_or(0);
                let unit = data.unit.as_str();
                msg.channel_id
                    .send_message(
                        &ctx.http,
                        CreateMessage::new()
                        .embed(
                            CreateEmbed::new()
                                .title("Wallet")
                                .description(format!("**{}**: {}", user_mention(msg.author.id), format_balance_with_unit(bal, unit)))
                                .color(0x00ff00),
                        )
                        .allowed_mentions(CreateAllowedMentions::new().empty_users()),
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

    fn version(&self) -> &'static str {
        "1.0.0"
    }

    fn subcommands(&self) -> &'static [SubCommandInfo] {
        static SUBCOMMANDS: &[SubCommandInfo] = &[
            SubCommandInfo {
                name: "check",
                description: "Check wallet balance (self or mentioned users)",
                aliases: &["bal", "balance"],
            },
            SubCommandInfo {
                name: "credit",
                description: "Add money (bot owner / server admin only)",
                aliases: &["add"],
            },
            SubCommandInfo {
                name: "debit",
                description: "Remove money (bot owner / server admin only)",
                aliases: &["remove", "sub"],
            },
            SubCommandInfo {
                name: "init",
                description: "Initialize wallet(s) (bot owner / server admin only)",
                aliases: &[],
            },
            SubCommandInfo {
                name: "reset",
                description: "Reset wallet(s) to balance (bot owner / server admin only)",
                aliases: &[],
            },
            SubCommandInfo {
                name: "unit",
                description: "Set display unit for balance (e.g. xu) (bot owner / server admin only)",
                aliases: &[],
            },
        ];
        SUBCOMMANDS
    }
}

async fn respond_ephemeral(interaction: &CommandInteraction, ctx: &Context, text: &str) -> anyhow::Result<()> {
    interaction
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                .content(text)
                .ephemeral(true)
                .allowed_mentions(CreateAllowedMentions::new().empty_users()),
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
                    .embed(CreateEmbed::new().title(title).description(description).color(color))
                    .allowed_mentions(CreateAllowedMentions::new().empty_users()),
            ),
        )
        .await?;
    Ok(())
}

pub fn create() -> Arc<dyn Command> {
    Arc::new(Wallet)
}
