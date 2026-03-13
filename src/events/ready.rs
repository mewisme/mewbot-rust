use crate::context::BotContext;
use serenity::model::gateway::Ready;
use serenity::prelude::Context;

pub async fn ready(ctx: Context, ready: Ready, bot_context: &BotContext) {
    crate::done!("{} is connected!", ready.user.name);

    match ctx.http.get_global_commands().await {
        Ok(existing_commands) => {
            for existing_cmd in existing_commands {
                if let Err(e) = ctx.http.delete_global_command(existing_cmd.id).await {
                    crate::error!(
                        "Failed to delete existing command {}: {}",
                        existing_cmd.name,
                        e
                    );
                }
            }
        }
        Err(e) => {
            crate::warn!(
                "Failed to fetch existing commands (may be first run): {}",
                e
            );
        }
    }

    crate::info!("Registering slash commands...");
    let registry = bot_context.registry.lock().await;
    let commands = registry.all_commands();

    for cmd in commands {
        let command_name = cmd.name();
        let mut create_cmd = serenity::builder::CreateCommand::new(command_name);
        cmd.register_slash(&mut create_cmd);

        match ctx.http.create_global_command(&create_cmd).await {
            Ok(_) => crate::done!("Registered slash command: {}", command_name),
            Err(e) => crate::error!("Failed to register slash command {}: {}", command_name, e),
        }
    }

    crate::done!("Ready! Bot is online.");
}
