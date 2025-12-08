use crate::context::BotContext;
use crate::utils;
use serenity::model::channel::Message;
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
