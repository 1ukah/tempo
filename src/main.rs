mod achievements;
mod guard;
mod idler;
mod status;
mod store;
mod ui;

use std::process::ExitCode;

use tokio::sync::watch;

const HELP: &str = "\
tempo: Steam hour farming from the command line

USAGE:
    tempo-cli [OPTIONS] <APP_ID>...

EXAMPLES:
    tempo-cli 730                 idle Counter-Strike 2
    tempo-cli 730 440 570         idle several games at once (up to 32)
    tempo-cli -u myaccount 730    pick the account when you have several saved
    tempo-cli -s away 730         idle while showing as Away (remembered for next time)
    tempo-cli -s invisible        just change the saved status, don't idle
    tempo-cli -a 620              idle and slowly unlock achievements like a real player

OPTIONS:
    -u, --user <NAME>    Steam account name (default: your only saved account)
    -s, --status <NAME>  Status friends see: online, away, busy, snooze, trade, play,
                         invisible, offline. Saved per account (default: online)
    -a, --achievements   Also unlock achievements over time, common ones first (rare and
                         server-only ones are skipped)
        --logout         Forget the saved login for the account and exit
    -h, --help           Show this help
    -V, --version        Show the version

App IDs are in the store URL: store.steampowered.com/app/<APP_ID>/
Your login is saved after the first sign-in (the password never is), in:
    Windows  %APPDATA%\\tempo
    macOS    ~/Library/Application Support/tempo
    Linux    ~/.config/tempo
Set TEMPO_HOME to use another folder, TEMPO_USER to set a default account.";

struct Args {
    user: Option<String>,
    logout: bool,
    status: Option<status::Status>,
    achievements: bool,
    apps: Vec<u32>,
}

fn parse_args() -> Result<Args, String> {
    let mut args = Args {
        user: None,
        logout: false,
        status: None,
        achievements: false,
        apps: Vec::new(),
    };
    let mut iter = std::env::args().skip(1);

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            "-V" | "--version" => {
                println!("tempo-cli {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "-u" | "--user" => args.user = Some(iter.next().ok_or("--user needs an account name")?),
            "--logout" => args.logout = true,
            "-a" | "--achievements" => args.achievements = true,
            "-s" | "--status" => {
                args.status = Some(parse_status(&iter.next().ok_or("--status needs a value")?)?);
            }
            _ if arg.starts_with("--status=") => {
                args.status = Some(parse_status(&arg["--status=".len()..])?);
            }
            _ if arg.starts_with("--user=") => args.user = Some(arg["--user=".len()..].to_owned()),
            _ if arg.starts_with('-') => {
                return Err(format!("unknown option '{arg}' (see --help)"));
            }
            _ => {
                for part in arg.split(',').filter(|p| !p.is_empty()) {
                    match part.trim().parse::<u32>() {
                        Ok(id) if id > 0 => {
                            if !args.apps.contains(&id) {
                                args.apps.push(id);
                            }
                        }
                        _ => return Err(format!("'{part}' is not a valid app ID")),
                    }
                }
            }
        }
    }

    if args.apps.len() > idler::MAX_GAMES {
        return Err(format!(
            "Steam allows at most {} games at once, you passed {}",
            idler::MAX_GAMES,
            args.apps.len()
        ));
    }

    if args.apps.is_empty() && !args.logout && args.status.is_none() {
        return Err("no app IDs given, e.g. `tempo-cli 730` (see --help)".into());
    }

    Ok(args)
}

fn parse_status(text: &str) -> Result<status::Status, String> {
    status::Status::parse(text).ok_or_else(|| {
        format!(
            "unknown status '{text}', use one of: {}",
            status::Status::all_names()
        )
    })
}

fn resolve_user(explicit: Option<String>) -> Result<String, String> {
    if let Some(user) = explicit
        .or_else(|| std::env::var("TEMPO_USER").ok())
        .filter(|u| !u.trim().is_empty())
    {
        return Ok(user.trim().to_owned());
    }

    let saved = store::saved_users();

    match saved.as_slice() {
        [only] => Ok(only.clone()),
        [] => {
            let user = ui::prompt("Steam account name: ")
                .map_err(|e| format!("couldn't read account name: {e}"))?;

            if user.is_empty() {
                Err("no account name given".into())
            } else {
                Ok(user)
            }
        }
        many => Err(format!(
            "several saved accounts ({}), pick one with --user <NAME>",
            many.join(", ")
        )),
    }
}

async fn termination_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

        match signal(SignalKind::terminate()) {
            Ok(mut term) => tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = term.recv() => {}
            },
            Err(_) => {
                let _ = tokio::signal::ctrl_c().await;
            }
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
    }
}

fn main() -> ExitCode {
    ui::init();

    let args = match parse_args() {
        Ok(args) => args,
        Err(e) => {
            ui::error(format!("error: {e}"));
            return ExitCode::from(2);
        }
    };

    let user = match resolve_user(args.user) {
        Ok(user) => user,
        Err(e) => {
            ui::error(format!("error: {e}"));
            return ExitCode::from(2);
        }
    };

    if args.logout {
        if store::delete_token(&user) {
            ui::ok(format!("Forgot the saved login for {user}."));
        } else {
            ui::info(format!("No saved login for {user}."));
        }

        return ExitCode::SUCCESS;
    }

    let status = match args.status {
        Some(status) => {
            if let Err(e) = store::save_status(&user, status.name()) {
                ui::warn(format!("Couldn't save the status for next time: {e}"));
            }

            if args.apps.is_empty() {
                ui::ok(format!(
                    "Status for {user} set to {}, it'll be used every time you idle.",
                    status.name()
                ));
                return ExitCode::SUCCESS;
            }

            status
        }
        None => store::load_status(&user)
            .and_then(|s| status::Status::parse(&s))
            .unwrap_or(status::DEFAULT),
    };

    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            ui::error(format!("error: couldn't start: {e}"));
            return ExitCode::FAILURE;
        }
    };

    let code = runtime.block_on(async move {
        let (stop_tx, stop_rx) = watch::channel(false);

        tokio::spawn(async move {
            termination_signal().await;
            let _ = stop_tx.send(true);

            termination_signal().await;
            std::process::exit(130);
        });

        match idler::Idler::new(user, args.apps, status, args.achievements, stop_rx)
            .run()
            .await
        {
            Ok(()) => 0,
            Err(e) => {
                ui::error(e);
                1
            }
        }
    });

    ui::status(None);
    std::process::exit(code);
}
