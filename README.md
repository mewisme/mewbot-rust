# Discord Bot - Rust

A Discord bot built with Rust using the Serenity framework. Supports both slash commands and prefix commands with cooldowns, auto-reload in dev mode, and a dynamic help system.

**Author:** Mew

## Features

- Unified command model supporting both slash and prefix commands
- Automatic command registration with Discord
- Per-user, per-command cooldown system with admin bypass
- Dynamic help system that auto-generates from registered commands
- Dev mode with file watching for command changes
- Modular architecture with clean separation of concerns

## Prerequisites

- Rust toolchain (latest stable version)
- Discord bot token
- Discord application ID

## Setup

1. Create a `.env` file in the root directory:

```env
DISCORD_TOKEN=your_bot_token_here
COMMAND_PREFIX=m/
DEV_MODE=false
ADMIN_USER_ID=
ENABLE_FILE_LINE_LOG=true
```

2. Get your Discord Bot Token:
   - Go to [Discord Developer Portal](https://discord.com/developers/applications)
   - Create or select an application
   - Navigate to "Bot" section
   - Copy the token and set as `DISCORD_TOKEN` in `.env`

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

- `DISCORD_TOKEN` - Discord bot token (required)
- `COMMAND_PREFIX` - Prefix for prefix commands (default: `m/`)
- `DEV_MODE` - Enable dev mode for file watching (default: `false`)
- `ADMIN_USER_ID` - User ID that bypasses cooldowns (optional)
- `ENABLE_FILE_LINE_LOG` - Show file:line in log output (default: `true`)

## CLI Commands

Generate a command template:

```bash
cargo run -- generate <command_name>
```

Show version:

```bash
cargo run -- version
```

Show config value:

```bash
cargo run -- config <key>
```

## Commands

### Using Commands

Slash Commands:
- Type `/` in Discord to see available commands
- Example: `/help`, `/flashback`

Prefix Commands:
- Use the configured prefix followed by the command name
- Example: `m/help`, `m/flashback`

## Adding New Commands

### Quick Method

Use the CLI generator:

```bash
cargo run -- generate mycommand
```

Then:
1. Edit the generated file in `src/commands/mycommand.rs`
2. Add `pub mod mycommand;` to `src/commands/mod.rs`
3. Add `registry.register(mycommand::create());` to `register_commands()` in `src/utils/mod.rs`
4. Rebuild and run

### Manual Method

1. Create a new file in `src/commands/` (e.g., `mycommand.rs`)

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
            .create_response(&ctx.http, CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .content("Response message")
            ))
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
        msg.channel_id
            .send_message(&ctx.http, |m| {
                m.content("Response message")
            })
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

3. Export the command in `src/commands/mod.rs`:
   - Add `pub mod mycommand;`

4. Register the command in `src/utils/mod.rs`:
   - Add `registry.register(mycommand::create());` in `register_commands()`

5. Rebuild and run

## Source Code Structure

```
src/
├── main.rs                 # Entry point, initializes bot and event handlers
├── cli/
│   └── mod.rs             # CLI command parsing and handlers
├── commands/              # Command implementations
│   ├── mod.rs            # Command trait definition
│   ├── flashback.rs      # Flashback command
│   └── help.rs           # Help command
├── config/
│   └── mod.rs            # Configuration loading from .env
├── context/
│   └── mod.rs            # Bot context (registry, cooldowns, config)
├── events/               # Discord event handlers
│   ├── mod.rs           # Event handler exports
│   ├── ready.rs         # Bot ready event (registers slash commands)
│   ├── message.rs       # Message event (handles prefix commands)
│   └── interaction.rs   # Interaction event (handles slash commands)
├── registry/
│   └── mod.rs           # Command registry for storing and retrieving commands
└── utils/               # Utility functions
    ├── mod.rs          # Helper functions (formatting, error handling)
    └── logger.rs       # Custom logging system
```

### Module Descriptions

**main.rs**
- Initializes the bot client
- Sets up event handlers
- Loads configuration
- Registers commands
- Starts the bot

**cli/mod.rs**
- Parses command-line arguments
- Handles CLI commands (generate, version, config)

**commands/**
- Defines the `Command` trait that all commands implement
- Each command file implements both slash and prefix command handlers
- Commands are registered in `utils/mod.rs`

**config/mod.rs**
- Loads configuration from environment variables
- Provides `Config` struct with all bot settings

**context/mod.rs**
- Manages shared bot state
- Handles cooldown tracking and checking
- Stores registry and configuration

**events/**
- `ready.rs`: Handles bot connection, registers slash commands with Discord
- `message.rs`: Processes prefix commands from messages
- `interaction.rs`: Processes slash command interactions

**registry/mod.rs**
- Stores all registered commands in HashMaps
- Provides fast lookup by command name or prefix
- Manages command aliases

**utils/**
- `mod.rs`: Utility functions for formatting, error messages, command registration
- `logger.rs`: Custom logging macros and formatting

## How It Works

### Command Registration

1. Commands are defined in `src/commands/` as separate modules
2. Each command exports a `create()` function returning `Arc<dyn Command>`
3. Commands are registered in `register_commands()` in `src/utils/mod.rs`
4. On bot startup, slash commands are registered with Discord in the `ready` event
5. The registry stores commands for fast lookup during execution

### Cooldown System

1. When a command executes, the bot checks if the user is on cooldown
2. Cooldowns are stored per user and per command
3. Admins (from `ADMIN_USER_ID`) bypass all cooldowns
4. If on cooldown, user receives an error message with remaining time
5. After successful execution, a new cooldown timestamp is set
6. Expired cooldowns are automatically cleaned up

### Help System

The help command dynamically queries the registry:
1. Calls `registry.all_commands()` to get all registered commands
2. Accesses command metadata (name, description, prefix, aliases, cooldown)
3. Formats information into an embed
4. Shows detailed information for specific commands when requested

## Dev Mode

When `DEV_MODE=true`, the bot watches `src/commands/` for file changes and logs when changes are detected. Note: Full hot-reload requires restarting the bot.

## Troubleshooting

Bot doesn't respond to commands:
- Verify `DISCORD_TOKEN` is correct
- Ensure bot has necessary permissions
- Verify `MESSAGE_CONTENT` intent is enabled for prefix commands
- Check console for error messages

Slash commands not appearing:
- Commands are registered on bot startup
- May take a few minutes to appear globally
- Check console for registration errors

## Dependencies

- serenity - Discord API wrapper
- tokio - Async runtime
- dotenv - Environment variable loading
- anyhow - Error handling
- notify - File watching
- chrono - Time formatting
- async-trait - Async trait support
- env_logger - Logging
- clap - CLI parsing
- reqwest - HTTP client

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

Copyright (c) 2024 Mew
