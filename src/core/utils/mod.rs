pub mod logger;

use chrono::{DateTime, Utc};
use serenity::model::channel::Message;
use std::time::Duration;

#[allow(dead_code)]
pub fn format_time_ago(dt: DateTime<Utc>) -> String {
    let now = Utc::now();
    let duration = now.signed_duration_since(dt);

    if duration.num_seconds() < 60 {
        format!("{} seconds ago", duration.num_seconds())
    } else if duration.num_minutes() < 60 {
        format!("{} minutes ago", duration.num_minutes())
    } else if duration.num_hours() < 24 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_days() < 30 {
        format!("{} days ago", duration.num_days())
    } else if duration.num_days() < 365 {
        format!("{} months ago", duration.num_days() / 30)
    } else {
        format!("{} years ago", duration.num_days() / 365)
    }
}

pub fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();

    if secs < 60 {
        format!("{} second{}", secs, if secs == 1 { "" } else { "s" })
    } else if secs < 3600 {
        let mins = secs / 60;
        format!("{} minute{}", mins, if mins == 1 { "" } else { "s" })
    } else {
        let hours = secs / 3600;
        format!("{} hour{}", hours, if hours == 1 { "" } else { "s" })
    }
}

pub fn format_error(error: &anyhow::Error) -> String {
    format!("Error: {}", error)
}

pub async fn send_error_message(msg: &Message, ctx: &serenity::prelude::Context, error: &str) {
    use serenity::builder::{CreateEmbed, CreateMessage};
    if let Err(e) = msg
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .title("Error")
                    .description(error)
                    .color(0xff0000),
            ),
        )
        .await
    {
        crate::core::utils::logger::log_internal(
            crate::core::utils::logger::LogLevel::Error,
            &format!("Failed to send error message: {}", e),
        );
    }
}

#[allow(dead_code)]
pub async fn send_success_message(msg: &Message, ctx: &serenity::prelude::Context, content: &str) {
    use serenity::builder::{CreateEmbed, CreateMessage};
    if let Err(e) = msg
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .title("Success")
                    .description(content)
                    .color(0x00ff00),
            ),
        )
        .await
    {
        crate::core::utils::logger::log_internal(
            crate::core::utils::logger::LogLevel::Error,
            &format!("Failed to send success message: {}", e),
        );
    }
}
