use crate::commands::Command;
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serenity::all::{CommandInteraction, CommandOptionType};
use serenity::builder::{
    CreateCommand, CreateCommandOption, CreateEmbed, CreateInteractionResponse,
    CreateInteractionResponseFollowup, CreateInteractionResponseMessage, CreateMessage,
};
use serenity::model::channel::Message;
use serenity::prelude::Context;
use std::sync::Arc;
use std::time::Duration;

const BASE_URL: &str = "https://eag6oc2f1c.execute-api.us-west-2.amazonaws.com";

#[derive(Debug, Deserialize, Serialize)]
struct FlashbackResponse {
    #[serde(rename = "isQualified")]
    is_qualified: Option<i32>,
    user: Option<UserData>,
}

#[derive(Debug, Deserialize, Serialize)]
struct UserData {
    #[serde(rename = "riot_id_name")]
    riot_id_name: Option<String>,
    #[serde(rename = "riot_id_tag")]
    riot_id_tag: Option<String>,
    #[serde(rename = "riot_id_combined")]
    riot_id_combined: Option<String>,
    #[serde(rename = "riot_id_combined_lower")]
    riot_id_combined_lower: Option<String>,
    qualified: Option<i32>,
    personal: Option<PersonalData>,
    stack: Option<StackData>,
    recommendations: Option<RecommendationsData>,
}

#[derive(Debug, Deserialize, Serialize)]
struct PersonalData {
    #[serde(rename = "most_played_agent")]
    most_played_agent: Option<String>,
    #[serde(rename = "best_agent")]
    best_agent: Option<String>,
    #[serde(rename = "worst_agent")]
    worst_agent: Option<String>,
    #[serde(rename = "agent_killed_most")]
    agent_killed_most: Option<String>,
    #[serde(rename = "agent_most_killed_by")]
    agent_most_killed_by: Option<String>,
    clutch_boast: Option<i32>,
    clutch_roast: Option<i32>,
    weapon_boast: Option<String>,
    weapon_roast: Option<String>,
    first_blood_kill: Option<i32>,
    first_blood_victim: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize)]
struct StackData {
    qualified: Option<i32>,
    win_partner: Option<String>,
    win_partner_agent: Option<String>,
    loss_partner: Option<String>,
    loss_partner_agent: Option<String>,
    most_assisted: Option<String>,
    most_assisted_agent: Option<String>,
    most_assisted_by: Option<String>,
    most_assisted_by_agent: Option<String>,
    headshot_highest: Option<String>,
    headshot_lowest: Option<String>,
    last_man_standing_most: Option<String>,
    last_man_standing_least: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct RecommendationsData {
    pro: Option<String>,
    influencer: Option<String>,
}

pub struct Flashback;

#[async_trait]
impl Command for Flashback {
    fn name(&self) -> &'static str {
        "flashback"
    }

    fn description(&self) -> &'static str {
        "Query Valorant player data by ID"
    }

    fn register_slash(&self, cmd: &mut CreateCommand) {
        *cmd = CreateCommand::new("flashback")
            .description("Query Valorant player data by ID")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::String,
                    "player-id",
                    "Player ID (Username#Tag)",
                )
                .required(true),
            );
    }

    async fn run_slash(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> anyhow::Result<()> {
        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Defer(
                    CreateInteractionResponseMessage::new().ephemeral(true),
                ),
            )
            .await?;

        let player_id = extract_player_id_slash(interaction)?;

        match call_flashback_api(&player_id).await {
            Ok(client_response) => {
                handle_flashback_response_slash(interaction, ctx, client_response, &player_id).await
            }
            Err(err) => {
                crate::error!(
                    "Error fetching flashback data for player '{}': {:?}",
                    player_id,
                    err
                );
                send_ephemeral(interaction, &ctx, "Failed to fetch player data.").await
            }
        }
    }

    fn prefix(&self) -> Option<&'static str> {
        Some("flashback")
    }

    async fn run_prefix(&self, ctx: &Context, msg: &Message, args: &[&str]) -> anyhow::Result<()> {
        if args.is_empty() {
            return send_prefix_error(ctx, msg, "Please provide a valid player ID!").await;
        }
        let player_id = args.join(" ");

        match call_flashback_api(&player_id).await {
            Ok(client_response) => {
                handle_flashback_response_prefix(ctx, msg, client_response, &player_id).await
            }
            Err(err) => {
                crate::error!(
                    "Error fetching flashback data for player '{}': {:?}",
                    player_id,
                    err
                );
                send_prefix_error(ctx, msg, "Failed to fetch player data.").await
            }
        }
    }

    fn cooldown_duration(&self) -> Duration {
        Duration::from_secs(3)
    }
}

fn extract_player_id_slash(interaction: &CommandInteraction) -> anyhow::Result<String> {
    for opt in &interaction.data.options {
        if opt.name == "player-id" {
            if let Some(resolved) = &opt.value.as_str() {
                return Ok(resolved.to_string());
            }
        }
    }

    anyhow::bail!("Missing or invalid player-id");
}

async fn call_flashback_api(player_id: &str) -> anyhow::Result<FlashbackResponse> {
    let client = Client::new();
    let api_url = format!("{}/dev/get-user", BASE_URL);

    let payload = serde_json::json!({
        "id": player_id,
        "env": "prod"
    });

    let res = client
        .post(&api_url)
        .header("Accept", "*/*")
        .header("Accept-Encoding", "gzip, deflate, br, zstd")
        .header("Accept-Language", "vi,en;q=0.9")
        .header("Cache-Control", "no-cache")
        .header("Content-Type", "application/json")
        .header("Origin", "https://flashback.playvalorant.com")
        .header("Pragma", "no-cache")
        .header("Referer", "https://flashback.playvalorant.com/")
        .header("Sec-Ch-Ua", "\"Chromium\";v=\"142\", \"Google Chrome\";v=\"142\", \"Not_A Brand\";v=\"99\"")
        .header("Sec-Ch-Ua-Mobile", "?0")
        .header("Sec-Ch-Ua-Platform", "\"Windows\"")
        .header("Sec-Fetch-Dest", "empty")
        .header("Sec-Fetch-Mode", "cors")
        .header("Sec-Fetch-Site", "cross-site")
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/142.0.0.0 Safari/537.36")
        .json(&payload)
        .send()
        .await?;

    let text = res.text().await?;
    Ok(serde_json::from_str(&text)?)
}

async fn handle_flashback_response_slash(
    interaction: &CommandInteraction,
    ctx: &Context,
    res: FlashbackResponse,
    player_id: &str,
) -> anyhow::Result<()> {
    if let Some(user) = res.user {
        let bot_user = ctx.http.get_current_user().await?;
        let embed =
            create_player_embed(&user, player_id, &bot_user.avatar_url().unwrap_or_default());

        interaction
            .create_followup(
                &ctx.http,
                CreateInteractionResponseFollowup::new().embed(embed),
            )
            .await?;
    } else {
        interaction
            .create_followup(
                &ctx.http,
                CreateInteractionResponseFollowup::new()
                    .content(format!("No player data found for `{}`.", player_id))
                    .ephemeral(true),
            )
            .await?;
    }

    Ok(())
}

async fn send_ephemeral(
    interaction: &CommandInteraction,
    ctx: &Context,
    msg: &str,
) -> anyhow::Result<()> {
    interaction
        .create_followup(
            &ctx.http,
            CreateInteractionResponseFollowup::new()
                .content(msg.to_string())
                .ephemeral(true),
        )
        .await?;
    Ok(())
}

async fn handle_flashback_response_prefix(
    ctx: &Context,
    msg: &Message,
    res: FlashbackResponse,
    player_id: &str,
) -> anyhow::Result<()> {
    if let Some(user) = res.user {
        let bot_user = ctx.http.get_current_user().await?;
        let embed =
            create_player_embed(&user, player_id, &bot_user.avatar_url().unwrap_or_default());

        msg.channel_id
            .send_message(&ctx.http, CreateMessage::new().embed(embed))
            .await?;
    } else {
        send_prefix_error(
            ctx,
            msg,
            &format!("No player data found for `{}`.", player_id),
        )
        .await?;
    }

    Ok(())
}

async fn send_prefix_error(ctx: &Context, msg: &Message, text: &str) -> anyhow::Result<()> {
    msg.channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .title("Error")
                    .description(text)
                    .color(0xff0000),
            ),
        )
        .await?;
    Ok(())
}

fn create_player_embed(user: &UserData, player_id: &str, bot_avatar: &str) -> CreateEmbed {
    let embed = CreateEmbed::new()
        .color(0xfa4454)
        .title("Valorant Flashback 2025")
        .url("https://flashback.playvalorant.com/")
        .thumbnail("https://flashback.playvalorant.com/apple-icon.png")
        .timestamp(chrono::Utc::now())
        .footer(serenity::builder::CreateEmbedFooter::new(player_id).icon_url(bot_avatar));

    let mut desc = String::new();

    /* ---------------- PLAYER ---------------- */
    if let Some(riot_id) = &user.riot_id_combined {
        desc.push_str(&format!("**Player:** {}\n\n", riot_id));
    }

    /* ---------------- PERSONAL ---------------- */
    if let Some(p) = &user.personal {
        desc.push_str("**Personal Highlights**\n");
        if let Some(agent) = &p.most_played_agent {
            desc.push_str(&format!("- Most Played: {}\n", agent));
        }
        if let Some(agent) = &p.best_agent {
            desc.push_str(&format!("- Best Agent: {}\n", agent));
        }
        if let Some(agent) = &p.worst_agent {
            desc.push_str(&format!("- Worst Agent: {}\n", agent));
        }
        if let Some(agent) = &p.agent_killed_most {
            desc.push_str(&format!("- Kills Most: {}\n", agent));
        }
        if let Some(agent) = &p.agent_most_killed_by {
            desc.push_str(&format!("- Killed By: {}\n", agent));
        }
        desc.push('\n');

        desc.push_str("**Combat Performance**\n");
        if let Some(clutch_boast) = p.clutch_boast {
            desc.push_str(&format!("- Clutch Boast: {}\n", clutch_boast));
        }
        if let Some(clutch_roast) = p.clutch_roast {
            desc.push_str(&format!("- Clutch Roast: {}\n", clutch_roast));
        }
        if let Some(weapon) = &p.weapon_boast {
            desc.push_str(&format!("- Best Weapon: {}\n", weapon));
        }
        if let Some(weapon) = &p.weapon_roast {
            desc.push_str(&format!("- Worst Weapon: {}\n", weapon));
        }
        if let Some(first_blood_kill) = p.first_blood_kill {
            desc.push_str(&format!("- First Bloods: {}\n", first_blood_kill));
        }
        if let Some(first_blood_victim) = p.first_blood_victim {
            desc.push_str(&format!("- First Deaths: {}\n", first_blood_victim));
        }
        desc.push('\n');
    }

    /* ---------------- TEAM ---------------- */
    if let Some(s) = &user.stack {
        desc.push_str("**Team Synergy**\n");
        if let Some(win_partner) = &s.win_partner {
            if let Some(agent) = &s.win_partner_agent {
                desc.push_str(&format!("- Win Partner: {} ({})\n", win_partner, agent));
            } else {
                desc.push_str(&format!("- Win Partner: {}\n", win_partner));
            }
        }
        if let Some(loss_partner) = &s.loss_partner {
            if let Some(agent) = &s.loss_partner_agent {
                desc.push_str(&format!("- Loss Partner: {} ({})\n", loss_partner, agent));
            } else {
                desc.push_str(&format!("- Loss Partner: {}\n", loss_partner));
            }
        }
        if let Some(most_assisted) = &s.most_assisted {
            if let Some(agent) = &s.most_assisted_agent {
                desc.push_str(&format!("- Most Assisted: {} ({})\n", most_assisted, agent));
            } else {
                desc.push_str(&format!("- Most Assisted: {}\n", most_assisted));
            }
        }
        if let Some(most_assisted_by) = &s.most_assisted_by {
            if let Some(agent) = &s.most_assisted_by_agent {
                desc.push_str(&format!(
                    "- Assisted By: {} ({})\n",
                    most_assisted_by, agent
                ));
            } else {
                desc.push_str(&format!("- Assisted By: {}\n", most_assisted_by));
            }
        }
        desc.push('\n');

        desc.push_str("**Headshot Insights**\n");
        if let Some(highest) = &s.headshot_highest {
            desc.push_str(&format!("- Highest HS%: {}\n", highest));
        }
        if let Some(lowest) = &s.headshot_lowest {
            desc.push_str(&format!("- Lowest HS%: {}\n", lowest));
        }
        if let Some(most) = &s.last_man_standing_most {
            desc.push_str(&format!("- Last Man Standing (Most): {}\n", most));
        }
        if let Some(least) = &s.last_man_standing_least {
            desc.push_str(&format!("- Last Man Standing (Least): {}\n", least));
        }
        desc.push('\n');
    }

    /* ---------------- RECOMMENDATIONS ---------------- */
    if let Some(r) = &user.recommendations {
        desc.push_str("**Community Picks**\n");
        if let Some(pro) = &r.pro {
            desc.push_str(&format!("- Pro: {}\n", pro));
        }
        if let Some(influencer) = &r.influencer {
            desc.push_str(&format!("- Influencer: {}\n", influencer));
        }
        desc.push('\n');
    }

    embed.description(desc)
}

pub fn create() -> Arc<dyn Command> {
    Arc::new(Flashback)
}
