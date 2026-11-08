# tempo

Steam hour farming from the command line.

Tempo farms playtime and achievements in your Steam games. You do not have to play the games. You do not need the Steam client or the games installed.

When you play a game on your account, tempo stops. When you close the game, tempo starts again. When you lose the internet connection, tempo waits for the connection and starts again.

## Install

1. Download the file for your computer from [Releases](../../releases).
2. Extract the file.
3. Do the install:
   - **Windows:** Double-click `install.cmd`.
   - **macOS and Linux:** Run `install.sh` from a terminal.
4. Open a new terminal. Type `tempo-cli --help`.

<details>
<summary>Which file do I download?</summary>

| Computer | File |
|---|---|
| Windows | `x86_64-pc-windows-msvc.zip` |
| Mac with Apple chip | `aarch64-apple-darwin.tar.gz` |
| Mac with Intel chip | `x86_64-apple-darwin.tar.gz` |
| Linux | `x86_64-unknown-linux-gnu.tar.gz` |

For ARM computers on Windows or Linux, use the `aarch64` file.

</details>

<details>
<summary>Windows shows a warning</summary>

Click **More info**. Then click **Run anyway**.

</details>

<details>
<summary>Use tempo without install</summary>

1. Open a terminal in the extracted folder.
2. Type `./tempo-cli` in place of `tempo-cli`. Example: `./tempo-cli 730`.

On macOS, first type `xattr -d com.apple.quarantine tempo-cli`.

</details>

## Use

```
tempo-cli 730
```

`730` is the app ID. Find it in the store URL (example: `store.steampowered.com/app/730/`) or on [SteamDB](https://steamdb.info).

At the first start, sign in with your password and Steam Guard. Tempo keeps a login token. It does not keep your password.

To stop, push Ctrl+C.

## Examples

| Command | Result |
|---|---|
| `tempo-cli 730 440` | Idle two games. The maximum is 32. |
| `tempo-cli -s away 730` | Idle and show "Away" to friends. |
| `tempo-cli -s invisible` | Change the status only. |
| `tempo-cli -a 620` | Idle and unlock achievements slowly. |
| `tempo-cli --logout` | Remove the saved login. |

## Options

| Option | Description |
|---|---|
| `-u`, `--user <NAME>` | Select the Steam account. |
| `-s`, `--status <NAME>` | Set the status: `online`, `away`, `busy`, `snooze`, `trade`, `play`, `invisible`, `offline`. Tempo keeps it. |
| `-a`, `--achievements` | Unlock achievements slowly. |
| `--logout` | Remove the saved login. |
| `-h`, `--help` | Show the help. |
| `-V`, `--version` | Show the version. |

## Saved data

| System | Folder |
|---|---|
| Windows | `%APPDATA%\tempo` |
| macOS | `~/Library/Application Support/tempo` |
| Linux | `~/.config/tempo` |

WARNING: The `.token` file gives access to your Steam account. Do not share it.

## Run from source

You must have [Rust](https://rustup.rs) and a C compiler.

```
git clone https://github.com/1ukah/tempo
cd tempo
cargo run --release -- 730
```

To install, do `cargo install --path .`

## License

[MIT](LICENSE)
