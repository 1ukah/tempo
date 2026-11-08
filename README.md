# tempo

Steam hour farming from the command line.

Tempo shows your Steam account as "in game". The games get playtime. You do not need the Steam client or the games.

## Install

Download the file for your system from [Releases](../../releases):

| System | File |
|---|---|
| Windows | `tempo-cli-x86_64-pc-windows-msvc.zip` |
| Windows on ARM | `tempo-cli-aarch64-pc-windows-msvc.zip` |
| macOS with Apple chip (M1 or newer) | `tempo-cli-aarch64-apple-darwin.tar.gz` |
| macOS with Intel chip | `tempo-cli-x86_64-apple-darwin.tar.gz` |
| Linux | `tempo-cli-x86_64-unknown-linux-gnu.tar.gz` |
| Linux on ARM | `tempo-cli-aarch64-unknown-linux-gnu.tar.gz` |

Extract the file. On Windows, right-click the zip file and select **Extract All**. On macOS, double-click the file.

Then select one of these two methods.

### Method A: Install (recommended)

After you install, you can use `tempo-cli` in every terminal, from every folder.

**Windows:** In the extracted folder, double-click `install.cmd`. If Windows shows "Windows protected your PC", click **More info**, then **Run anyway**.

**macOS and Linux:**

1. Open the Terminal app.
2. Type `sh ` (with a space at the end). Then drag `install.sh` from the extracted folder into the Terminal window.
3. Push Enter.

When the install is done, close the terminal and open a new one. Type `tempo-cli --help` to make sure that it works.

The install puts `tempo-cli` in this folder:

| System | Folder |
|---|---|
| Windows | `%LOCALAPPDATA%\Programs\tempo` |
| macOS and Linux | `~/.local/bin` |

To install a new version, download it and do the install again. To remove tempo, delete `tempo-cli` from this folder.

### Method B: Use without install

Open a terminal in the extracted folder:

- **Windows:** Open the folder. Right-click an empty area in the folder and select **Open in Terminal**. On Windows 10, hold Shift, right-click, and select **Open PowerShell window here**.
- **macOS:** Open the Terminal app. Type `cd ` (with a space at the end), drag the folder into the Terminal window, and push Enter. At the first start, type `xattr -d com.apple.quarantine tempo-cli` and push Enter. This lets macOS run the file.
- **Linux:** Right-click an empty area in the folder and select **Open in Terminal**.

Then type `./tempo-cli` instead of `tempo-cli` in the commands below. For example, `./tempo-cli 730`.

With this method, you must always open the terminal in this folder.

## Use

```
tempo-cli 730
```

`730` is the app ID. Find it in the store URL: `store.steampowered.com/app/730/`.

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

## How it works

- **Pause:** When you play a game on your account, tempo stops. When you close the game, tempo starts again.
- **Reconnect:** When the connection stops, tempo connects again.
- **Status:** Tempo keeps your status for each account. The default is `online`.
- **Achievements:** Tempo unlocks one achievement every 30 to 150 minutes. Common achievements unlock first. Tempo does not unlock rare achievements. Use this only for games that you do not play.

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
