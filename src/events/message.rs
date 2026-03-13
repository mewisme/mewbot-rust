use crate::context::BotContext;
use crate::permissions::{get_permission_level, has_permission, PermissionLevel};
use crate::utils;
use serenity::model::channel::Message;
use serenity::model::permissions::Permissions;
use serenity::prelude::Context;

pub async fn message(ctx: Context, msg: Message, bot_context: &BotContext) {
    if msg.author.bot {
        return;
    }

    let prefix = &bot_context.config.command_prefix;

    if !msg.content.starts_with(prefix) {
        return;
    }

    let content = msg.content.strip_prefix(prefix).unwrap_or("");
    let parts: Vec<&str> = content.split_whitespace().collect();

    if parts.is_empty() {
        return;
    }

    let command_name = parts[0];
    let args = &parts[1..];

    let registry = bot_context.registry.lock().await;
    let command = match registry.get_prefix_command(command_name) {
        Some(cmd) => cmd,
        None => return,
    };
    drop(registry);

    if let Some(required) = command.required_permission_level() {
        let guild_id = match msg.guild_id {
            Some(id) => id,
            None => {
                utils::send_error_message(
                    &msg,
                    &ctx,
                    "This command can only be used in a server.",
                )
                .await;
                return;
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
                utils::send_error_message(&msg, &ctx, "Could not load server information.")
                    .await;
                return;
            }
        };
        let user_level = get_permission_level(owner_id, msg.author.id, has_administrator);
        if !has_permission(user_level, required) {
            let required_str = match required {
                PermissionLevel::Owner => "server owner",
                PermissionLevel::Admin => "server admin or owner",
                PermissionLevel::Member => "member",
            };
            utils::send_error_message(
                &msg,
                &ctx,
                &format!("This command requires **{}** permission.", required_str),
            )
            .await;
            return;
        }
    }

    let user_id = msg.author.id.get();
    let cooldown_duration = command.cooldown_duration();

    if let Some(remaining) = bot_context
        .get_cooldown_remaining(user_id, command.name(), cooldown_duration)
        .await
    {
        let remaining_str = utils::format_duration(remaining);
        let error_msg = format!("You are on cooldown for {} more seconds", remaining_str);
        utils::send_error_message(&msg, &ctx, &error_msg).await;
        return;
    }

    match command.run_prefix(&ctx, &msg, args).await {
        Ok(_) => {
            bot_context.set_cooldown(user_id, command.name()).await;
        }
        Err(e) => {
            let error_msg = utils::format_error(&e);
            utils::send_error_message(&msg, &ctx, &error_msg).await;
        }
    }
}
