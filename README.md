# tempo

[![Release](https://img.shields.io/github/v/release/1ukah/tempo)](../../releases)
[![CI](https://github.com/1ukah/tempo/actions/workflows/ci.yml/badge.svg)](../../actions/workflows/ci.yml)
[![License](https://img.shields.io/github/license/1ukah/tempo)](LICENSE)

**Farm Steam playtime and achievements from your terminal.** No Steam client. No game downloads.

- Idle up to 32 games at the same time
- Unlock achievements slowly, common ones first
- Pauses when you play a game, and starts again when you stop
- Waits for the internet connection when it goes down, and starts again
- Works on Windows, macOS and Linux

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

`730` is the game's app ID. Find it in the store URL (example: `store.steampowered.com/app/730/`) or on [SteamDB](https://steamdb.info).

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
