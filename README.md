# Discord Bot (Rust)

A Discord bot built with Rust using the Serenity framework. It uses a **core + plugins** architecture: each command is a plugin. The bot supports slash commands and prefix commands, per-command cooldowns, a permission hierarchy (bot owner, server admin, member), and a help system that can show command and subcommand details.

**Author:** Mew

## Features

- **Core + plugins** – Commands live in `src/plugins/<name>/`; core provides config, registry, permissions, and a limited plugin API.
- **Unified command model** – Same commands work as slash and prefix.
- **Automatic slash registration** – Slash commands are registered with Discord on ready.
- **Per-user, per-command cooldown** – Bot owner bypass via `ADMIN_USER_ID`.
- **Permission hierarchy** – **Bot owner** (all servers) > **server admin** (per server) > member.
- **Help** – `/help` lists commands; `/help <command>` shows command + subcommands; `/help <command> <subcommand>` (e.g. `/help wallet check`) shows subcommand help.
- **Wallet plugin** – check, credit, debit, init, reset, unit (JSON-backed); user mentions (no ping).
- **Plugin version** – Each plugin has a version; "Loaded plugin X vY" is logged at startup.
- **CLI** – `generate-plugin <name>` creates a new plugin template and wires it into the bot.

## Prerequisites

- Rust toolchain (latest stable)
- Discord bot token
- For permission checks and wallet init-all: **Server Members Intent** enabled in the Discord Developer Portal

## Setup

1. Create a `.env` file in the project root:

```env
DISCORD_TOKEN=your_bot_token_here
COMMAND_PREFIX=m/
ADMIN_USER_ID=
```

2. Get your Discord bot token from the [Discord Developer Portal](https://discord.com/developers/applications) (Bot section).

3. Build and run:

```bash
cargo build --release
cargo run
```

For development: `cargo run`

## Configuration

| Variable           | Description                                      |
|--------------------|--------------------------------------------------|
| `DISCORD_TOKEN`    | Discord bot token (required)                     |
| `COMMAND_PREFIX`   | Prefix for text commands (default: `m/`)         |
| `ADMIN_USER_ID`    | User ID of the **bot owner** (optional, recommended). Bypasses cooldowns and can use all commands in every server. |

**Permission levels**

- **Bot owner** – User in `ADMIN_USER_ID`. Highest level; all commands in any server.
- **Server admin** – Guild owner or users with **Administrator** in that server only.
- **Member** – Everyone else.

Enable **Server Members Intent** (Bot → Privileged Gateway Intents) in the Discord Developer Portal for permission checks and wallet init-all.

## Scripts

- **`scripts/update-version.ps1`** – Update the package version in `Cargo.toml`. Argument: an exact semver (e.g. `1.2.3`) or `major` / `minor` / `patch` to bump the current version.

  ```powershell
  .\scripts\update-version.ps1 patch   # 0.3.4 -> 0.3.5
  .\scripts\update-version.ps1 minor  # 0.3.4 -> 0.4.0
  .\scripts\update-version.ps1 major   # 0.3.4 -> 1.0.0
  .\scripts\update-version.ps1 1.0.0   # set exact version
  ```

## CLI commands

Generate a new plugin (command) template:

```bash
cargo run -- generate-plugin <name>
```

Example: `cargo run -- generate-plugin my_command` creates `src/plugins/my_command/` with `mod.rs` and `command.rs`, and updates `src/plugins/mod.rs`.

Show version:

```bash
cargo run -- version
```

Show an env/config value:

```bash
cargo run -- config <key>
```

## Bot commands

**Slash** – Type `/` (e.g. `/help`, `/wallet`).  
**Prefix** – Use prefix + command (e.g. `m/help`, `m/wallet`). Aliases (e.g. `m/w`, `m/bal` for wallet) work when defined.

### Help

- **`/help`** or **`m/help`** (aliases: `m/h`, `m/commands`) – List all commands with short descriptions.
- **`/help <command>`** or **`m/help <command>`** – Help for one command: description, slash/prefix, aliases, cooldown, permission, version, and **list of subcommands** (if any).
- **`/help <command> <subcommand>`** or **`m/help <command> <subcommand>`** – Help for a specific subcommand (e.g. `/help wallet check`).

### Wallet

- **check** (default) – View balance. Self or (with permission) others. **Bot owner** or **server admin** can view others.
- **credit** – Add balance. **Bot owner** or **server admin** only.
- **debit** – Remove balance. Same permission as credit.
- **init** – Initialize wallet(s), default balance 0. Optional user; no user = init all non-bot members in the server.
- **reset** – Set wallet(s) to a given balance. Same permission and user rules as init.
- **unit** – Set display unit for balances (e.g. "xu"). Same permission.

Data is stored in `data/wallet.json`.

## Adding new commands (plugins)

### Using the CLI

```bash
cargo run -- generate-plugin mycommand
```

Then edit `src/plugins/mycommand/command.rs` to implement your logic. The CLI already adds the module and registration in `src/plugins/mod.rs`. Rebuild and run.

### Manual method

1. Create `src/plugins/<name>/mod.rs` and `src/plugins/<name>/command.rs`.
2. In `command.rs`, implement the `Command` trait from `crate::core::Command` (name, description, register_slash, run_slash, prefix, run_prefix; optionally aliases, cooldown_duration, required_permission_level, version, subcommands).
3. In `src/plugins/mod.rs`: add `mod <name>;` and in `register_commands()` create the command, register it, and log "Loaded plugin ... v...".
4. Rebuild and run.

Plugins can use `crate::core::permissions` for permission checks. The help plugin uses `PluginApi` (config + command_lister); other plugins usually only need the trait and permissions.

## Source code structure

```
src/
├── main.rs                 # Entry point, core + plugins, client and event handler
├── cli/
│   └── mod.rs              # CLI: generate-plugin, version, config
├── core/
│   ├── mod.rs              # Re-exports (Command, Config, PluginApi, Registry, etc.)
│   ├── config.rs           # Config from environment
│   ├── context.rs          # BotContext (config, registry, cooldowns)
│   ├── registry.rs         # Registry + RegistryCommandLister (for help)
│   ├── command.rs          # Command trait, SubCommandInfo, CommandInfo, CommandLister
│   ├── permissions.rs      # Owner/Admin/Member and helpers
│   ├── plugin_api.rs      # PluginApi (config + command_lister)
│   ├── events/
│   │   ├── mod.rs
│   │   ├── ready.rs       # Register slash commands with Discord
│   │   ├── message.rs     # Prefix command dispatch
│   │   └── interaction.rs # Slash command dispatch
│   ├── utils/
│   │   ├── mod.rs         # Formatting, send_error_message, etc.
│   │   └── logger.rs      # Logging macros (info!, done!, error!, warn!)
│   └── updater.rs         # Auto-update from GitHub releases
└── plugins/
    ├── mod.rs             # register_commands(registry, api); loads wallet then help
    ├── help/
    │   ├── mod.rs
    │   └── command.rs     # Help: list all, command+subcommands, or single subcommand
    └── wallet/
        ├── mod.rs
        ├── command.rs     # Wallet slash + prefix (check, credit, debit, init, reset, unit)
        └── store.rs       # Wallet JSON load/save (data/wallet.json)
```

- **core** – Config, registry, context, command trait, permissions, plugin API, events, utils, updater. Plugins depend only on the public API and permissions.
- **plugins** – Each plugin is a folder (e.g. `wallet/`) with `mod.rs`, `command.rs`, and optional files (e.g. `store.rs`). Registered in `plugins::register_commands`; help is registered last so it can list all commands.

## How it works

- **Registration** – `main` builds `Registry`, `BotContext`, and `PluginApi` (config + command_lister). It calls `plugins::register_commands(&mut reg, api)`, which creates each plugin command and registers it. On ready, slash commands are sent to Discord.
- **Cooldowns** – Per user, per command; bot owner skips. Success sets cooldown; old entries are cleared over time.
- **Help** – Uses `PluginApi::command_lister` to get `CommandInfo` (name, description, subcommands, etc.). Parses "command" / "command subcommand" to show list, command+subcommands, or single subcommand embed.

## Troubleshooting

- **Bot does not respond** – Check `DISCORD_TOKEN`, bot permissions, and (for prefix) **MESSAGE_CONTENT** intent.
- **Slash commands missing** – They are registered on ready; global updates can take a moment. Check console for errors.
- **Permission or init-all issues** – Enable **Server Members Intent** in the Discord Developer Portal.

## Dependencies

- serenity – Discord API
- tokio – Async runtime
- dotenv – `.env` loading
- anyhow – Error handling
- chrono – Time formatting
- async-trait – Async trait support
- env_logger – Logging
- clap – CLI
- reqwest – HTTP (updater)
- serde / serde_json – Serialization (wallet, etc.)

## License

MIT License. See [LICENSE](LICENSE).

Copyright (c) 2024 Mew
