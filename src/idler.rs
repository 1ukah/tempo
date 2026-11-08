use std::collections::HashMap;
use std::pin::pin;
use std::time::{Duration, Instant};

use steam_vent::auth::{
    AuthConfirmationHandler, DeviceConfirmationHandler, FileGuardDataStore,
    UserProvidedAuthConfirmationHandler,
};
use steam_vent::{Connection, ConnectionError, ConnectionTrait, EResult, LoginError, ServerList};
use steam_vent_proto_common::protobuf::EnumOrUnknown;
use steam_vent_proto_steam::enums_clientserver::EMsg;
use steam_vent_proto_steam::steammessages_auth_steamclient::{
    CAuthentication_AccessToken_GenerateForApp_Request, ETokenRenewalType,
};
use steam_vent_proto_steam::steammessages_clientserver::CMsgClientGamesPlayed;
use steam_vent_proto_steam::steammessages_clientserver::cmsg_client_games_played::GamePlayed;
use steam_vent_proto_steam::steammessages_clientserver_2::CMsgClientPlayingSessionState;
use steam_vent_proto_steam::steammessages_clientserver_friends::CMsgClientChangeStatus;
use steam_vent_proto_steam::steammessages_clientserver_login::CMsgClientLoggedOff;
use steam_vent_proto_steam::steammessages_player_steamclient::{
    CPlayer_GetNicknameList_Request, CPlayer_GetOwnedGames_Request,
};
use tokio::sync::watch;
use tokio::time::{interval, sleep, timeout};
use tokio_stream::StreamExt;

use crate::achievements::{self, Plan, Rng};
use crate::guard::GuardPrompt;
use crate::status::Status;
use crate::{store, ui};

pub const MAX_GAMES: usize = 32;

const RECONNECT_DELAYS: [u64; 6] = [5, 10, 30, 60, 120, 300];
const RATE_LIMIT_DELAY: Duration = Duration::from_secs(25 * 60);
const PROBE_INTERVAL: Duration = Duration::from_secs(30);
const PROBE_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_PASSWORD_FAILURES: u8 = 3;

enum Outcome {
    Retry,
    RetryNow,
    RateLimited,
    Shutdown,
    Fatal(String),
}

struct Game {
    name: String,
    minutes: u32,
}

pub struct Idler {
    user: String,
    apps: Vec<u32>,
    status: Status,
    shutdown: watch::Receiver<bool>,
    password: Option<String>,
    password_failures: u8,
    games: Option<HashMap<u32, Game>>,
    idled: Duration,
    idling_since: Option<Instant>,
    signed_in: bool,
    achievements: bool,
    plans: Option<Vec<Plan>>,
    rng: Rng,
}

impl Idler {
    pub fn new(
        user: String,
        apps: Vec<u32>,
        status: Status,
        achievements: bool,
        shutdown: watch::Receiver<bool>,
    ) -> Self {
        Self {
            user,
            apps,
            status,
            shutdown,
            password: None,
            password_failures: 0,
            games: None,
            idled: Duration::ZERO,
            idling_since: None,
            signed_in: false,
            achievements,
            plans: None,
            rng: Rng::new(),
        }
    }

    pub async fn run(mut self) -> Result<(), String> {
        let mut attempt = 0usize;

        let result = loop {
            let delay = match self.session().await {
                Outcome::Shutdown => break Ok(()),
                Outcome::Fatal(message) => break Err(message),
                Outcome::RetryNow => {
                    attempt = 0;
                    continue;
                }
                Outcome::RateLimited => {
                    ui::warn(format!(
                        "Steam is rate limiting sign-ins from your IP, waiting {} minutes...",
                        RATE_LIMIT_DELAY.as_secs() / 60
                    ));
                    RATE_LIMIT_DELAY
                }
                Outcome::Retry => {
                    if std::mem::take(&mut self.signed_in) {
                        attempt = 0;
                    }

                    let secs = RECONNECT_DELAYS[attempt.min(RECONNECT_DELAYS.len() - 1)];
                    attempt += 1;
                    ui::info(format!("Reconnecting in {secs}s..."));
                    Duration::from_secs(secs)
                }
            };

            tokio::select! {
                _ = sleep(delay) => {}
                _ = self.shutdown.changed() => break Ok(()),
            }
        };

        self.pause();

        if self.idled > Duration::ZERO {
            ui::info(format!(
                "Idled for {} in total. Bye!",
                ui::duration(self.idled)
            ));
        }

        result
    }

    async fn session(&mut self) -> Outcome {
        let mut shutdown = self.shutdown.clone();

        let connection = tokio::select! {
            biased;
            _ = shutdown.changed() => return Outcome::Shutdown,
            result = self.sign_in() => match result {
                Ok(connection) => connection,
                Err(outcome) => return outcome,
            },
        };

        self.idle(connection).await
    }

    async fn sign_in(&mut self) -> Result<Connection, Outcome> {
        if store::load_token(&self.user).is_none() && self.password.is_none() {
            let question = format!("Steam password for {}: ", self.user);
            let password = tokio::task::spawn_blocking(move || ui::prompt_password(&question))
                .await
                .map_err(|e| Outcome::Fatal(e.to_string()))?
                .map_err(|e| Outcome::Fatal(format!("Couldn't read password: {e}")))?;

            if password.is_empty() {
                return Err(Outcome::Fatal("No password given.".into()));
            }

            self.password = Some(password);
        }

        ui::info("Connecting to Steam...");

        let servers = ServerList::discover().await.map_err(|e| {
            ui::warn(format!("Couldn't reach Steam: {e}"));
            Outcome::Retry
        })?;

        if let Some(token) = store::load_token(&self.user) {
            return match Connection::access(&servers, &self.user, &token).await {
                Ok(connection) => Ok(connection),
                Err(e) if token_rejected(&e) => {
                    ui::warn("Your saved login expired or was revoked, please sign in again.");
                    store::delete_token(&self.user);
                    Err(Outcome::RetryNow)
                }
                Err(ConnectionError::LoginError(LoginError::RateLimited)) => {
                    Err(Outcome::RateLimited)
                }
                Err(e) => {
                    ui::warn(format!("Sign-in failed: {e}"));
                    Err(Outcome::Retry)
                }
            };
        }

        let password = self.password.clone().unwrap_or_default();
        ui::info("Signing in...");

        let result = Connection::login(
            &servers,
            &self.user,
            &password,
            FileGuardDataStore::new(store::guard_data_path()),
            UserProvidedAuthConfirmationHandler::new(tokio::io::stdin(), GuardPrompt::default())
                .or(DeviceConfirmationHandler),
        )
        .await;

        match result {
            Ok(connection) => {
                self.password = None;

                match connection.access_token() {
                    Some(token) => match store::save_token(&self.user, token) {
                        Ok(()) => ui::ok(format!(
                            "Login saved in {} (your password is not stored).",
                            store::dir().display()
                        )),
                        Err(e) => ui::warn(format!(
                            "Couldn't save login, you'll be asked again next time: {e}"
                        )),
                    },
                    None => ui::warn(
                        "Steam didn't return a login token; you'll be asked again next time.",
                    ),
                }

                Ok(connection)
            }
            Err(ConnectionError::LoginError(LoginError::InvalidCredentials)) => {
                self.password = None;
                self.password_failures += 1;

                if self.password_failures >= MAX_PASSWORD_FAILURES {
                    return Err(Outcome::Fatal(
                        "Wrong username or password too many times, giving up.".into(),
                    ));
                }

                ui::error("Wrong username or password.");
                Err(Outcome::RetryNow)
            }
            Err(ConnectionError::LoginError(LoginError::RateLimited)) => Err(Outcome::RateLimited),
            Err(ConnectionError::Aborted) => Err(Outcome::Fatal(
                "Sign-in aborted (no Steam Guard code entered).".into(),
            )),
            Err(ConnectionError::UnsupportedConfirmationAction(_)) => Err(Outcome::Fatal(
                "Steam asked for a sign-in confirmation tempo does not support.".into(),
            )),
            Err(e) => {
                ui::warn(format!("Sign-in failed: {e}"));
                Err(Outcome::Retry)
            }
        }
    }

    async fn idle(&mut self, connection: Connection) -> Outcome {
        let mut playing_state = pin!(connection.on::<CMsgClientPlayingSessionState>());
        let mut logged_off = pin!(connection.on::<CMsgClientLoggedOff>());

        ui::ok(format!("Signed in as {}.", self.user));
        self.signed_in = true;

        let change = CMsgClientChangeStatus {
            persona_state: Some(self.status.persona_state()),
            persona_set_by_user: Some(true),
            ..Default::default()
        };

        match connection.send(change).await {
            Ok(()) => ui::info(format!("Status: {}", self.status.name())),
            Err(e) => ui::warn(format!("Couldn't set status: {e}")),
        }

        self.renew_token(&connection).await;
        self.load_games(&connection).await;

        if let Some(missing) = self.not_owned() {
            return Outcome::Fatal(missing);
        }

        let mut blocked = matches!(
            timeout(Duration::from_secs(2), playing_state.next()).await,
            Ok(Some(Ok(state))) if state.playing_blocked()
        );

        if blocked {
            ui::warn("You're playing a game on another device. Idling starts once you stop.");
        } else if let Err(outcome) = self.start(&connection).await {
            return outcome;
        }

        if self.achievements && self.plans.is_none() {
            self.prepare_achievements(&connection).await;
        }

        let mut achievement_tick = interval(Duration::from_secs(30));
        let mut probe = interval(PROBE_INTERVAL);
        probe.tick().await;
        let mut probe_failures = 0u8;
        let mut ticker = interval(Duration::from_secs(1));

        loop {
            tokio::select! {
                _ = self.shutdown.changed() => {
                    self.pause();
                    ui::info("Stopping...");
                    let _ = timeout(Duration::from_secs(3), send_games_played(&connection, &[])).await;
                    return Outcome::Shutdown;
                }
                Some(state) = playing_state.next() => {
                    let Ok(state) = state else { continue };

                    if state.playing_blocked() == blocked {
                        continue;
                    }

                    blocked = state.playing_blocked();

                    if blocked {
                        self.pause();
                        ui::warn("You started playing on another device, pausing. Idling resumes when you stop.");
                    } else {
                        ui::info("Your other game session ended, resuming.");

                        if let Err(outcome) = self.start(&connection).await {
                            return outcome;
                        }
                    }
                }
                Some(message) = logged_off.next() => {
                    self.pause();
                    let reason = message.map(|m| m.eresult()).unwrap_or_default();
                    ui::warn(format!("Steam logged us off (code {reason})."));
                    return Outcome::Retry;
                }
                _ = probe.tick() => {
                    let alive = matches!(
                        timeout(PROBE_TIMEOUT, connection.service_method(CPlayer_GetNicknameList_Request::new())).await,
                        Ok(Ok(_))
                    );

                    probe_failures = if alive { 0 } else { probe_failures + 1 };

                    if probe_failures >= 2 {
                        self.pause();
                        ui::warn("Lost connection to Steam.");
                        return Outcome::Retry;
                    }
                }
                _ = achievement_tick.tick(), if self.plans.as_ref().is_some_and(|p| !p.is_empty()) => {
                    if !blocked {
                        self.farm_achievements(&connection).await;
                    }
                }
                _ = ticker.tick() => {
                    if let Some(since) = self.idling_since {
                        ui::status(Some(format!(
                            "Idling {} for {} (Ctrl+C to stop)",
                            games_label(self.apps.len()),
                            ui::duration(self.idled + since.elapsed())
                        )));
                    }
                }
            }
        }
    }

    async fn prepare_achievements(&mut self, connection: &Connection) {
        let mut plans = Vec::new();

        for &app in &self.apps {
            let game = self
                .games
                .as_ref()
                .and_then(|g| g.get(&app))
                .map_or_else(|| format!("app {app}"), |g| g.name.clone());

            if let Some(plan) = Plan::prepare(connection, app, &game, &mut self.rng).await {
                plans.push(plan);
            }
        }

        self.plans = Some(plans);
    }

    async fn farm_achievements(&mut self, connection: &Connection) {
        let Some(since) = self.idling_since else {
            return;
        };
        let idled = self.idled + since.elapsed();
        let Some(plans) = self.plans.as_mut() else {
            return;
        };

        for plan in plans.iter_mut() {
            if plan.is_due(idled) {
                plan.unlock_next(connection, idled, &mut self.rng).await;
            }
        }

        plans.retain(|p| !p.is_done());
    }

    async fn start(&mut self, connection: &Connection) -> Result<(), Outcome> {
        if let Err(e) = send_games_played(connection, &self.apps).await {
            ui::warn(format!("Couldn't tell Steam what we're playing: {e}"));
            return Err(Outcome::Retry);
        }

        self.idling_since.get_or_insert_with(Instant::now);
        ui::ok(format!("Idling {}:", games_label(self.apps.len())));

        let idled_minutes = u32::try_from(self.idled.as_secs() / 60).unwrap_or(u32::MAX);

        for &id in &self.apps {
            let progress = achievements::progress(connection, id).await;
            let mut line = match self.games.as_ref().and_then(|g| g.get(&id)) {
                Some(game) => format!(
                    "  \u{2022} {} ({id}): {} played",
                    game.name,
                    playtime(game.minutes.saturating_add(idled_minutes))
                ),
                None => format!("  \u{2022} App {id}"),
            };

            if let Some((have, total)) = progress {
                line.push_str(&format!(", {have}/{total} achievements"));
            }

            ui::info(line);
        }

        Ok(())
    }

    fn pause(&mut self) {
        if let Some(since) = self.idling_since.take() {
            self.idled += since.elapsed();
        }

        ui::status(None);
    }

    fn not_owned(&self) -> Option<String> {
        let games = self.games.as_ref()?;
        let missing: Vec<String> = self
            .apps
            .iter()
            .filter(|id| !games.contains_key(id))
            .map(u32::to_string)
            .collect();

        match missing.as_slice() {
            [] => None,
            [one] => Some(format!(
                "This account does not own app ID {one}. Check the app ID in the store URL."
            )),
            many => Some(format!(
                "This account does not own app IDs {}. Check the app IDs in the store URL.",
                many.join(", ")
            )),
        }
    }

    async fn load_games(&mut self, connection: &Connection) {
        if self.games.is_some() {
            return;
        }

        let request = CPlayer_GetOwnedGames_Request {
            steamid: Some(connection.steam_id().into()),
            include_appinfo: Some(true),
            include_played_free_games: Some(true),
            include_free_sub: Some(true),
            appids_filter: self.apps.clone(),
            ..Default::default()
        };

        if let Ok(Ok(response)) =
            timeout(Duration::from_secs(10), connection.service_method(request)).await
        {
            self.games = Some(
                response
                    .games
                    .into_iter()
                    .map(|g| {
                        (
                            g.appid() as u32,
                            Game {
                                name: g.name().to_owned(),
                                minutes: g.playtime_forever() as u32,
                            },
                        )
                    })
                    .collect(),
            );
        }
    }

    async fn renew_token(&self, connection: &Connection) {
        let Some(token) = store::load_token(&self.user) else {
            return;
        };

        let request = CAuthentication_AccessToken_GenerateForApp_Request {
            refresh_token: Some(token.clone()),
            steamid: Some(connection.steam_id().into()),
            renewal_type: Some(EnumOrUnknown::new(
                ETokenRenewalType::k_ETokenRenewalType_Allow,
            )),
            ..Default::default()
        };

        if let Ok(Ok(response)) =
            timeout(Duration::from_secs(10), connection.service_method(request)).await
        {
            let renewed = response.refresh_token();

            if !renewed.is_empty() && renewed != token {
                let _ = store::save_token(&self.user, renewed);
            }
        }
    }
}

fn games_label(count: usize) -> String {
    if count == 1 {
        "1 game".into()
    } else {
        format!("{count} games")
    }
}

fn playtime(minutes: u32) -> String {
    match minutes {
        0..60 => format!("{minutes}m"),
        _ => format!("{}h {:02}m", minutes / 60, minutes % 60),
    }
}

async fn send_games_played(
    connection: &Connection,
    apps: &[u32],
) -> Result<(), steam_vent::NetworkError> {
    let message = CMsgClientGamesPlayed {
        games_played: apps
            .iter()
            .map(|&id| GamePlayed {
                game_id: Some(u64::from(id)),
                ..Default::default()
            })
            .collect(),
        client_os_type: Some(os_type()),
        ..Default::default()
    };

    connection
        .send_with_kind(message, EMsg::k_EMsgClientGamesPlayedWithDataBlob)
        .await
}

fn os_type() -> u32 {
    let value: i32 = if cfg!(windows) {
        16
    } else if cfg!(target_os = "macos") {
        -102
    } else {
        -203
    };

    value as u32
}

fn token_rejected(error: &ConnectionError) -> bool {
    match error {
        ConnectionError::AccessToken(_) => true,
        ConnectionError::LoginError(LoginError::Unknown(result)) => !matches!(
            result,
            EResult::Fail
                | EResult::NoConnection
                | EResult::Busy
                | EResult::Timeout
                | EResult::ServiceUnavailable
                | EResult::Pending
                | EResult::TryAnotherCM
                | EResult::Cancelled
                | EResult::TooManyPending
        ),
        ConnectionError::LoginError(
            LoginError::InvalidCredentials | LoginError::InvalidSteamId,
        ) => true,
        _ => false,
    }
}
