# Changelog

All important changes to tempo are in this file.
The release workflow uses the section of each version as the release notes.

## [0.2.0]

### Added

- Show a list of the idled games with their names, playtime and achievements.

### Fixed

- Show a clear question for the Steam Guard code.
- Stop with an error when the account does not own an app ID.

## [0.1.0]

### Added

- Idle up to 32 Steam games at the same time.
- Sign in one time with password and Steam Guard. Tempo keeps a login token, not the password.
- Pause automatically when you play a game on the same account. Resume when you stop.
- Reconnect automatically when the connection to Steam stops.
- Set the online status with `--status`. Tempo keeps the status for each account.
- Unlock achievements slowly with `--achievements`. Common achievements unlock first.
- Remove the saved login with `--logout`.
- Builds for Windows, macOS and Linux (x64 and ARM64).
