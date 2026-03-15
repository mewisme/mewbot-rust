# Discord Bot (Rust)

A Discord bot built with Rust using the Serenity framework. It supports both slash commands and prefix commands, per-command cooldowns, a permission hierarchy (bot owner, server admin, member), and a dynamic help system.

**Author:** Mew

## Features

- Unified command model: same commands work as slash and prefix
- Automatic slash command registration with Discord
- Per-user, per-command cooldown (bot owner bypass via `ADMIN_USER_ID`)
- Permission hierarchy: **bot owner** (all servers) > **server admin** (per server) > member
- Dynamic help: `help` lists commands; `help <command>` shows detailed usage (slash, prefix, aliases, cooldown, permission)
- Wallet command: check, credit, debit, init, reset (JSON-backed); user mentions in messages (no ping)
- Modular layout with clear separation of concerns

## Prerequisites

- Rust toolchain (latest stable)
- Discord bot token and application ID
- For permission checks and wallet init-all: Server Members Intent enabled in the Discord Developer Portal

## Setup

1. Create a `.env` file in the project root:

```env
DISCORD_TOKEN=your_bot_token_here
COMMAND_PREFIX=m/
ADMIN_USER_ID=
```

2. Get your Discord bot token:
   - Open the [Discord Developer Portal](https://discord.com/developers/applications)
   - Create or select an application
   - Go to the "Bot" section and copy the token into `DISCORD_TOKEN` in `.env`

3. Build and run:

```bash
cargo build --release
cargo run
```

For development:

```bash
cargo run
```

## Configuration

Environment variables:

- `DISCORD_TOKEN` – Discord bot token (required)
- `COMMAND_PREFIX` – Prefix for text commands (default: `m/`)
- `ADMIN_USER_ID` – User ID of the **bot owner**. This user can use all commands in every server (including ones they don’t own) and bypasses cooldowns. Optional but recommended.

### Permission hierarchy (bot owner > server admin > member)

- **Bot owner** – The user whose ID is set in `ADMIN_USER_ID`. Can use every command in **any server**. Highest level.
- **Server admin** – Guild owner or users with the **Administrator** permission **in that server only**. Can use admin-only commands only within that server.
- **Member** – All other users.

The bot uses cached guild and member data for permission checks, so enable the **Server Members Intent** (`GUILD_MEMBERS`) in the [Discord Developer Portal](https://discord.com/developers/applications) (Bot → Privileged Gateway Intents).

Commands can require a minimum level via `required_permission_level()` on the Command trait. If unset, the command is available to all members.

## CLI commands

Generate a new command template:

```bash
cargo run -- generate <command_name>
```

Print version:

```bash
cargo run -- version
```

Print a config value (from environment):

```bash
cargo run -- config <key>
```

## Bot commands

### Usage

**Slash commands** – Type `/` in Discord (e.g. `/help`, `/wallet`).

**Prefix commands** – Use the configured prefix plus the command name (e.g. `m/help`, `m/wallet`). Aliases (e.g. `m/w`, `m/bal` for wallet) work when defined.

### Help command

- **`/help`** or **`m/help`** (aliases: `m/h`, `m/commands`) – Lists all available commands with short descriptions.
- **`/help <command>`** or **`m/help <command>`** – Shows detailed help for one command:
  - **Description** – What the command does.
  - **Slash** – Slash usage (e.g. `/wallet`).
  - **Prefix** – Prefix usage (e.g. `m/wallet`).
  - **Aliases** – Other ways to call it (e.g. `m/w`, `m/bal`, `m/balance`).
  - **Cooldown** – Seconds between uses.
  - **Permission** – Who can use it (any member, server admin, or bot owner).

Example: `m/help wallet` or `/help command:wallet` for full wallet help.

### Wallet command

- **check** (default) – View balance. No user option: your wallet. With user/mention(s): those users’ wallets. Only **bot owner** or **server admin** can view others’ wallets; members can only view their own. Balances and user names use mentions (no ping).
- **credit** – Add balance. **Bot owner** or **server admin** only. Optional user; default self.
- **debit** – Remove balance. Same permission and user rules as credit.
- **init** – Initialize wallet(s), default balance 0. **Bot owner** or **server admin** only. Optional user; **no user = init all non-bot members** in the server (cached member list).
- **reset** – Set wallet(s) to a given balance. Same permission and user rules as init.

Data is stored in `data/wallet.json` (JSON, keyed by user ID).

## Adding new commands

### Quick method

Use the CLI generator:

```bash
cargo run -- generate mycommand
```

Then:

1. Edit the generated file under `src/commands/mycommand.rs`
2. Add `pub mod mycommand;` in `src/commands/mod.rs`
3. Add `registry.register(mycommand::create());` in `register_commands()` in `src/utils/mod.rs`
4. Rebuild and run

### Manual method

1. Add a new file under `src/commands/` (e.g. `mycommand.rs`).

2. Implement the `Command` trait:

```rust
use crate::commands::Command;
use async_trait::async_trait;
use serenity::builder::CreateCommand;
use serenity::model::application::CommandInteraction;
use serenity::model::channel::Message;
use serenity::prelude::Context;
use std::sync::Arc;
use std::time::Duration;

pub struct MyCommand;

#[async_trait]
impl Command for MyCommand {
    fn name(&self) -> &'static str {
        "mycommand"
    }

    fn description(&self) -> &'static str {
        "Description of my command"
    }

    fn register_slash(&self, cmd: &mut CreateCommand) {
        *cmd = CreateCommand::new("mycommand")
            .description("Description of my command");
    }

    async fn run_slash(
        &self,
        ctx: &Context,
        interaction: &CommandInteraction,
    ) -> anyhow::Result<()> {
        use serenity::builder::CreateInteractionResponse;
        use serenity::builder::CreateInteractionResponseMessage;

        interaction
            .create_response(
                &ctx.http,
                CreateInteractionResponse::Message(
                    CreateInteractionResponseMessage::new().content("Response message"),
                ),
            )
            .await?;
        Ok(())
    }

    fn prefix(&self) -> Option<&'static str> {
        Some("mycommand")
    }

    async fn run_prefix(
        &self,
        ctx: &Context,
        msg: &Message,
        args: &[&str],
    ) -> anyhow::Result<()> {
        use serenity::builder::CreateMessage;
        msg.channel_id
            .send_message(&ctx.http, CreateMessage::new().content("Response message"))
            .await?;
        Ok(())
    }

    fn cooldown_duration(&self) -> Duration {
        Duration::from_secs(3)
    }
}

pub fn create() -> Arc<dyn Command> {
    Arc::new(MyCommand)
}
```

3. In `src/commands/mod.rs`, add: `pub mod mycommand;`
4. In `src/utils/mod.rs`, inside `register_commands()`, add: `registry.register(mycommand::create());`
5. Rebuild and run

## Source code structure

```
src/
├── main.rs              # Entry point, client and event handler setup
├── cli/
│   └── mod.rs           # CLI parsing and handlers (generate, version, config)
├── commands/
│   ├── mod.rs           # Command trait and command exports
│   ├── help.rs         # Help command
│   └── wallet.rs       # Wallet command (check, add, remove, init)
├── config/
│   └── mod.rs          # Config from environment
├── context/
│   └── mod.rs          # Shared state (registry, cooldowns, config)
├── events/
│   ├── mod.rs
│   ├── ready.rs        # Ready: register slash commands
│   ├── message.rs      # Prefix command handling
│   └── interaction.rs  # Slash command handling
├── permissions/
│   └── mod.rs          # Permission level (Owner, Admin, Member) and helpers
├── registry/
│   └── mod.rs          # Command registry (by name and prefix)
├── utils/
│   ├── mod.rs          # Helpers, formatting, command registration
│   └── logger.rs      # Logging macros
└── wallet_store.rs     # Wallet JSON load/save and data types
```

### Module overview

- **main.rs** – Loads config, builds registry and context, registers commands, starts the client with event handlers.
- **cli/mod.rs** – Handles CLI subcommands: generate, version, config.
- **commands/** – Command trait and implementations; each command can define slash and prefix behavior, aliases, and cooldown.
- **config/mod.rs** – Reads `.env` into a `Config` struct.
- **context/mod.rs** – Holds registry, config, and per-user cooldown state.
- **events/** – `ready`: register global slash commands; `message`: dispatch prefix commands; `interaction`: dispatch slash commands and enforce permissions.
- **permissions/mod.rs** – Defines Owner/Admin/Member and helpers for permission checks using guild cache.
- **registry/mod.rs** – Registers commands and looks them up by name or prefix (including aliases).
- **utils/mod.rs** – Formatting, error handling, and `register_commands()`.
- **wallet_store.rs** – Wallet data (JSON file), load/save, and init/add/remove balance helpers.

## How it works

### Command registration

1. Commands live in `src/commands/` and export a `create()` returning `Arc<dyn Command>`.
2. They are registered in `register_commands()` in `src/utils/mod.rs`.
3. On startup, the `ready` event registers slash commands with Discord and the in-memory registry is used for both slash and prefix dispatch.

### Cooldowns

- Cooldowns are per user and per command.
- Users in `ADMIN_USER_ID` skip cooldowns.
- If a user is on cooldown, they get a message with the remaining time. After a successful run, cooldown is set; old entries are cleared over time.

### Help

The help command uses the registry to list commands. With no argument it shows all commands and a short description. With a command name (e.g. `help wallet`) it shows detailed info in a structured embed: description, slash usage, prefix usage, aliases, cooldown, and required permission level.

## Troubleshooting

**Bot does not respond to commands**

- Check `DISCORD_TOKEN` and bot permissions.
- For prefix commands, ensure the `MESSAGE_CONTENT` intent is enabled.
- Check logs for errors.

**Slash commands do not show up**

- Slash commands are registered when the bot becomes ready; global updates can take a short time.
- Look for registration errors in the console.

**Permission or “init all” issues**

- Enable the Server Members Intent in the Discord Developer Portal (Bot > Privileged Gateway Intents) so the bot can use cached members for permission checks and wallet init-all.

## Dependencies

- serenity – Discord API and gateway
- tokio – Async runtime
- dotenv – Load `.env`
- anyhow – Error handling
- chrono – Time formatting
- async-trait – Async trait support
- env_logger – Logging
- clap – CLI
- reqwest – HTTP client
- serde / serde_json – Serialization (e.g. wallet JSON)

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.

Copyright (c) 2024 Mew
