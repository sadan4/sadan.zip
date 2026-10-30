# discord-mcp

An [MCP](https://modelcontextprotocol.io) server for writing Discord client mods
(Vencord and similar) against Discord's webpack bundle. It lets an AI agent
load a Discord build, read and search modules, generate finds, look up intl
strings, and test a plugin's patches, all offline without a running Discord
client.

It talks MCP over stdio, so any MCP client that can launch a local command can
use it.

## Install

### Prebuilt binary

Grab the binary for your platform from the
[nightly release](https://github.com/sadan4/sadan.zip/releases/tag/nightly):

- Linux: `discord-mcp-x86_64-unknown-linux-gnu`
- Windows: `discord-mcp-x86_64-pc-windows-msvc.exe`

Put it somewhere permanent and note the absolute path. On Linux, make it
executable:

```sh
mv discord-mcp-x86_64-unknown-linux-gnu ~/.local/bin/discord-mcp
chmod +x ~/.local/bin/discord-mcp
```

### From source

Needs a nightly Rust toolchain. Building from inside the checkout picks it up
from `rust-toolchain.toml`.

```sh
git clone https://github.com/sadan4/sadan.zip
cd sadan.zip
cargo build --release -p discord_mcp
```

The binary ends up at `target/release/discord-mcp` (`discord-mcp.exe` on
Windows).

## Add it to your client

Replace `/path/to/discord-mcp` below with the absolute path to the binary.

### Claude Code

```sh
claude mcp add discord -- /path/to/discord-mcp
```

Add `--scope user` to make it available in every project, not just the current
one.

### Claude Desktop, Cursor and other `mcpServers` clients

Add an entry to the client's MCP config (`claude_desktop_config.json` for
Claude Desktop, `~/.cursor/mcp.json` for Cursor):

```json
{
    "mcpServers": {
        "discord": {
            "command": "/path/to/discord-mcp"
        }
    }
}
```

On Windows, escape backslashes in the path (`"C:\\tools\\discord-mcp.exe"`).

### VS Code

Add to `.vscode/mcp.json` in your workspace (or the user-level `mcp.json`):

```json
{
    "servers": {
        "discord": {
            "type": "stdio",
            "command": "/path/to/discord-mcp"
        }
    }
}
```

## Usage

Ask the agent to load a build first. `load_build` with no hash pulls the newest
build archived by the explorer server; it can also scrape the live stable or
canary build from discord.com, or load a `.modules` directory written by the
companion LSP. Every other tool works on the most recently loaded build.

A typical patch workflow: `search_modules` / `find_module` to locate code,
`get_module` to read it, `generate_finds` for a unique find, then `test_patch`
to apply a plugin's patches and see the diff or errors. `check_plugins` runs
every plugin in a Vencord checkout against the build, like the reporter.

`module_exports`, `find_export_uses`, `find_store` and `css_classes` parse the
whole build, which takes a while the first time per build.

## Configuration

All optional, set through the client's `env` config:

| Variable                   | Default                     | Purpose                                                             |
| -------------------------- | --------------------------- | ------------------------------------------------------------------- |
| `DISCORD_MCP_EXPLORER_URL` | `https://s-d-br.sadan.zip`  | Explorer server to fetch archived builds from, eg a local instance |
| `DISCORD_MCP_LOG`          | `info`                      | Log filter (`tracing` `EnvFilter` syntax). Logs go to stderr       |
| `REPORTER_CACHE_DIR`       | `$XDG_CACHE_HOME/reporter`  | Where scraped builds are cached on disk                            |

`$XDG_CACHE_HOME` falls back to `~/.cache`.

For example, in an `mcpServers` config:

```json
{
    "mcpServers": {
        "discord": {
            "command": "/path/to/discord-mcp",
            "env": {
                "DISCORD_MCP_LOG": "debug"
            }
        }
    }
}
```

With Claude Code, pass `-e DISCORD_MCP_LOG=debug` to `claude mcp add`.
