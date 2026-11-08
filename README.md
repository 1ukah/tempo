# tempo

Tempo is a command-line tool for Steam hour farming.

Tempo shows your Steam account as "in game" for the games that you select. These games then get playtime. You do not need the Steam client. You do not need to install the games.

Tempo is small and light. It is one file of approximately 3 MB and it uses little memory. It operates on Windows, macOS and Linux.

## Features

### Idle games

Tempo idles up to 32 games at the same time. Give the app ID of each game. The app ID is in the store URL of the game. For example, `store.steampowered.com/app/730/` has the app ID `730`.

### Sign in

At the first start, tempo asks for your account name, your password and your Steam Guard code. You can also approve the sign-in in the Steam Mobile app.

Tempo then keeps a login token. Tempo does not keep your password. At the next start, tempo uses the token and does not ask for your password again. When Steam gives a new token, tempo replaces the old token.

### Automatic pause

When you play a game on the same account, Steam tells tempo. Tempo then stops idling, so that Steam does not stop your game. When you close your game, tempo starts to idle again. This applies to a game on this computer and to a game on a different device.

### Automatic reconnect

When the connection to Steam stops, tempo connects again. The time between two attempts increases from 5 seconds to 5 minutes. When Steam limits the sign-ins, tempo waits 25 minutes.

### Online status

Use `--status` to set the status that your friends see. Tempo keeps the status for each account. Tempo then uses this status at each start. The default status is `online`. The games get playtime with all the statuses.

### Achievements

Use `--achievements` to unlock the achievements of the idled games. Tempo unlocks them slowly, as a real player does:

- Tempo unlocks one achievement at a time.
- The first achievement unlocks after 20 to 60 minutes of idle time. Then one achievement unlocks every 30 to 150 minutes. The time is random.
- Only the idle time counts. A pause or a new start does not make the unlocks faster.
- Common achievements unlock first more frequently. Tempo uses the global unlock percentage of each achievement.
- Tempo does not unlock rare achievements. An achievement is rare when it is 10 times less common than the most common achievement of the game.
- Tempo does not unlock achievements that only official game servers can set. Some online games have only this type of achievement.

Use this feature only for games that you do not play. Some achievements are related to the progress in the game. If you play the game later, the game can reset these achievements.

## Install

1. Download the archive for your system from the [Releases](../../releases) page.
2. Extract the archive.
3. Put the `tempo-cli` file in a folder that is in your `PATH`. On Windows, the file is `tempo-cli.exe`.

| System | Archive |
|---|---|
| Windows x64 | `tempo-cli-x86_64-pc-windows-msvc.zip` |
| Windows ARM64 | `tempo-cli-aarch64-pc-windows-msvc.zip` |
| macOS Apple silicon | `tempo-cli-aarch64-apple-darwin.tar.gz` |
| macOS Intel | `tempo-cli-x86_64-apple-darwin.tar.gz` |
| Linux x64 | `tempo-cli-x86_64-unknown-linux-gnu.tar.gz` |
| Linux ARM64 | `tempo-cli-aarch64-unknown-linux-gnu.tar.gz` |

The `SHA256SUMS` file on the Releases page has the checksum of each archive.

## Usage

```
tempo-cli [OPTIONS] <APP_ID>...
```

Examples:

```
tempo-cli 730
tempo-cli 730 440 570
tempo-cli -s away 730
tempo-cli -a 620
tempo-cli -s invisible
tempo-cli --logout
```

To stop tempo, push Ctrl+C. To stop tempo immediately, push Ctrl+C two times.

## Options

| Option | Description |
|---|---|
| `-u`, `--user <NAME>` | Use this Steam account. The default is the only saved account. |
| `-s`, `--status <NAME>` | Set and keep the online status: `online`, `away`, `busy`, `snooze`, `trade`, `play`, `invisible` or `offline`. Without app IDs, tempo only keeps the status and stops. |
| `-a`, `--achievements` | Unlock the achievements of the idled games slowly. |
| `--logout` | Remove the saved login of the account. |
| `-h`, `--help` | Show the help. |
| `-V`, `--version` | Show the version. |

| Environment variable | Description |
|---|---|
| `TEMPO_USER` | The default Steam account. |
| `TEMPO_HOME` | The folder for the saved data. |

## Saved data

Tempo keeps the login token and the status of each account in this folder:

| System | Folder |
|---|---|
| Windows | `%APPDATA%\tempo` |
| macOS | `~/Library/Application Support/tempo` |
| Linux | `$XDG_CONFIG_HOME/tempo` or `~/.config/tempo` |

WARNING: The `.token` file gives access to your Steam account. Do not give this file to other persons.

## Run from source

You must have [Rust](https://rustup.rs) (stable) and a C compiler.

1. Get the source code:

   ```
   git clone <repository-url> tempo
   cd tempo
   ```

2. Run tempo:

   ```
   cargo run --release -- 730
   ```

3. To install `tempo-cli` in your `PATH`, do this command:

   ```
   cargo install --path .
   ```

## License

MIT. Refer to [LICENSE](LICENSE).
