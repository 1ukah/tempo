use std::collections::HashMap;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use steam_vent::{Connection, ConnectionTrait};
use steam_vent_proto_steam::steammessages_clientserver_userstats::{
    CMsgClientGetUserStats, CMsgClientGetUserStatsResponse, CMsgClientStoreUserStats2,
    CMsgClientStoreUserStatsResponse, cmsg_client_store_user_stats2,
};
use tokio::time::timeout;

use crate::ui;

const RARE_RATIO: f32 = 0.10;
const FIRST_UNLOCK_MINUTES: (u64, u64) = (20, 60);
const UNLOCK_GAP_MINUTES: (u64, u64) = (30, 150);
const RETRY_LATER: Duration = Duration::from_secs(10 * 60);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(20);

struct Candidate {
    stat_id: u32,
    bit: u32,
    name: String,
    percent: f32,
}

pub struct Plan {
    app: u32,
    game: String,
    candidates: Vec<Candidate>,
    schema_version: i32,
    next_at: Duration,
}

impl Plan {
    pub async fn prepare(
        connection: &Connection,
        app: u32,
        game: &str,
        rng: &mut Rng,
    ) -> Option<Self> {
        let Some(stats) = get_user_stats(connection, app, -1).await else {
            ui::warn(format!(
                "{game}: couldn't load achievements, skipping them."
            ));
            return None;
        };

        let Some(schema) = stats.schema.as_deref().and_then(kv::parse) else {
            ui::info(format!("{game}: no achievements."));
            return None;
        };

        let definitions = achievements_in(&schema);

        if definitions.is_empty() {
            ui::info(format!("{game}: no achievements."));
            return None;
        }

        let percents = global_percentages(app).await;
        let unlocked = unlocked_bits(&stats);
        let max_percent = percents.values().copied().fold(0.0f32, f32::max);
        let rare_below = max_percent * RARE_RATIO;

        let (mut have, mut protected, mut rare) = (0, 0, 0);
        let mut candidates = Vec::new();

        for def in definitions {
            if unlocked.contains(&(def.stat_id, def.bit)) {
                have += 1;
            } else if def.protected {
                protected += 1;
            } else {
                match percents.get(&def.api_name) {
                    Some(&percent) if percent > 0.0 && percent >= rare_below => {
                        candidates.push(Candidate {
                            stat_id: def.stat_id,
                            bit: def.bit,
                            name: def.display_name,
                            percent,
                        })
                    }
                    _ => rare += 1,
                }
            }
        }

        let total = have + protected + rare + candidates.len();
        let mut skipped = Vec::new();

        if rare > 0 {
            skipped.push(format!("{rare} rare"));
        }

        if protected > 0 {
            skipped.push(format!("{protected} server-only"));
        }

        let skipped = if skipped.is_empty() {
            String::new()
        } else {
            format!(", skipping {}", skipped.join(" and "))
        };

        if candidates.is_empty() {
            ui::info(format!(
                "{game}: {have}/{total} achievements, nothing left to farm{skipped}."
            ));
            return None;
        }

        ui::info(format!(
            "{game}: {have}/{total} achievements, {} will unlock over time{skipped}.",
            candidates.len()
        ));

        Some(Self {
            app,
            game: game.to_owned(),
            candidates,
            schema_version: schema_version(&schema),
            next_at: rng.minutes(FIRST_UNLOCK_MINUTES),
        })
    }

    pub fn is_due(&self, idled: Duration) -> bool {
        !self.candidates.is_empty() && idled >= self.next_at
    }

    pub fn is_done(&self) -> bool {
        self.candidates.is_empty()
    }

    pub async fn unlock_next(&mut self, connection: &Connection, idled: Duration, rng: &mut Rng) {
        let Some(stats) = get_user_stats(connection, self.app, self.schema_version).await else {
            self.next_at = idled + RETRY_LATER;
            return;
        };

        let unlocked = unlocked_bits(&stats);
        self.candidates
            .retain(|c| !unlocked.contains(&(c.stat_id, c.bit)));

        let Some(index) = rng.weighted(self.candidates.iter().map(|c| c.percent * c.percent))
        else {
            return;
        };

        let target = &self.candidates[index];

        let current = stats
            .stats
            .iter()
            .find(|s| s.stat_id() == target.stat_id)
            .map_or(0, |s| s.stat_value());

        let request = CMsgClientStoreUserStats2 {
            game_id: Some(u64::from(self.app)),
            settor_steam_id: Some(connection.steam_id().into()),
            settee_steam_id: Some(connection.steam_id().into()),
            crc_stats: Some(stats.crc_stats()),
            explicit_reset: Some(false),
            stats: vec![cmsg_client_store_user_stats2::Stats {
                stat_id: Some(target.stat_id),
                stat_value: Some(current | (1 << target.bit)),
                ..Default::default()
            }],
            ..Default::default()
        };

        let response =
            request_response::<_, CMsgClientStoreUserStatsResponse>(connection, request).await;

        match response {
            Some(r) if r.eresult() == 1 && r.stats_failed_validation.is_empty() => {
                let done = self.candidates.swap_remove(index);
                ui::ok(format!(
                    "{}: unlocked \"{}\" ({:.1}% of players)",
                    self.game, done.name, done.percent
                ));

                if self.candidates.is_empty() {
                    ui::info(format!(
                        "{}: every farmable achievement is unlocked.",
                        self.game
                    ));
                }

                self.next_at = idled + rng.minutes(UNLOCK_GAP_MINUTES);
            }
            Some(r) if !r.stats_failed_validation.is_empty() || r.eresult() == 15 => {
                let dropped = self.candidates.swap_remove(index);
                ui::warn(format!(
                    "{}: Steam refused \"{}\", skipping it.",
                    self.game, dropped.name
                ));
                self.next_at = idled + rng.minutes(FIRST_UNLOCK_MINUTES);
            }
            _ => self.next_at = idled + RETRY_LATER,
        }
    }
}

struct Definition {
    stat_id: u32,
    bit: u32,
    api_name: String,
    display_name: String,
    protected: bool,
}

fn achievements_in(schema: &kv::Kv) -> Vec<Definition> {
    let Some(stats) = schema
        .get("stats")
        .or_else(|| schema.first_section().and_then(|s| s.get("stats")))
    else {
        return Vec::new();
    };

    let mut out = Vec::new();

    for (key, stat) in stats.children() {
        let is_achievement_block = matches!(
            stat.get("type").map(kv::Kv::as_text).as_deref(),
            Some("4" | "5" | "ACHIEVEMENTS" | "GROUPACHIEVEMENTS")
        );

        let Some(bits) = stat.get("bits").filter(|_| is_achievement_block) else {
            continue;
        };
        let Some(stat_id) = stat
            .get("id")
            .and_then(kv::Kv::as_u32)
            .or_else(|| key.parse().ok())
        else {
            continue;
        };
        let stat_protected = stat.get("permission").and_then(kv::Kv::as_u32).unwrap_or(0) & 2 != 0;

        for (bit_key, def) in bits.children() {
            let Some(bit) = def
                .get("bit")
                .and_then(kv::Kv::as_u32)
                .or_else(|| bit_key.parse().ok())
            else {
                continue;
            };
            let Some(api_name) = def
                .get("name")
                .map(kv::Kv::as_text)
                .filter(|n| !n.is_empty())
            else {
                continue;
            };

            if bit >= 32 {
                continue;
            }

            let display_name = def
                .get("display")
                .and_then(|d| d.get("name"))
                .map(|n| {
                    n.get("english")
                        .map_or_else(|| n.as_text(), kv::Kv::as_text)
                })
                .filter(|n| !n.is_empty())
                .unwrap_or_else(|| api_name.clone());

            out.push(Definition {
                stat_id,
                bit,
                display_name,
                protected: stat_protected
                    || def.get("permission").and_then(kv::Kv::as_u32).unwrap_or(0) & 2 != 0,
                api_name,
            });
        }
    }

    out
}

fn schema_version(schema: &kv::Kv) -> i32 {
    schema
        .get("version")
        .or_else(|| schema.first_section().and_then(|s| s.get("version")))
        .and_then(kv::Kv::as_u32)
        .map_or(-1, |v| v as i32)
}

fn unlocked_bits(stats: &CMsgClientGetUserStatsResponse) -> Vec<(u32, u32)> {
    let mut out = Vec::new();

    for stat in &stats.stats {
        for bit in 0..32 {
            if stat.stat_value() & (1 << bit) != 0 {
                out.push((stat.stat_id(), bit));
            }
        }
    }

    for block in &stats.achievement_blocks {
        for (bit, &time) in block.unlock_time.iter().enumerate() {
            if time != 0 {
                out.push((block.achievement_id(), bit as u32));
            }
        }
    }

    out
}

async fn get_user_stats(
    connection: &Connection,
    app: u32,
    schema_version: i32,
) -> Option<CMsgClientGetUserStatsResponse> {
    let request = CMsgClientGetUserStats {
        game_id: Some(u64::from(app)),
        schema_local_version: Some(schema_version),
        steam_id_for_user: Some(connection.steam_id().into()),
        ..Default::default()
    };

    request_response::<_, CMsgClientGetUserStatsResponse>(connection, request)
        .await
        .filter(|r| r.eresult() == 1)
}

async fn request_response<Req, Rsp>(connection: &Connection, request: Req) -> Option<Rsp>
where
    Req: steam_vent::NetMessage,
    Rsp: steam_vent::NetMessage + 'static,
{
    let by_kind = connection.one::<Rsp>();
    let by_job = connection.job::<Req, Rsp>(request);

    let result = timeout(REQUEST_TIMEOUT, async {
        tokio::select! {
            r = by_kind => r,
            r = by_job => r,
        }
    })
    .await;

    result.ok()?.ok()
}

async fn global_percentages(app: u32) -> HashMap<String, f32> {
    let url = format!(
        "https://api.steampowered.com/ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/?gameid={app}"
    );

    let Ok(Ok(response)) = timeout(REQUEST_TIMEOUT, reqwest::get(&url)).await else {
        return HashMap::new();
    };

    let Ok(body) = response.text().await else {
        return HashMap::new();
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) else {
        return HashMap::new();
    };

    json["achievementpercentages"]["achievements"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|a| {
            let name = a["name"].as_str()?.to_owned();
            let percent = match &a["percent"] {
                serde_json::Value::String(s) => s.parse().ok()?,
                v => v.as_f64()? as f32,
            };
            Some((name, percent))
        })
        .collect()
}

pub struct Rng(u64);

impl Rng {
    pub fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos() as u64);
        Self((nanos ^ u64::from(std::process::id()).rotate_left(32)) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }

    fn minutes(&mut self, (min, max): (u64, u64)) -> Duration {
        Duration::from_secs(60 * (min + self.next() % (max - min + 1)))
    }

    fn weighted(&mut self, weights: impl Iterator<Item = f32> + Clone) -> Option<usize> {
        let total: f64 = weights.clone().map(f64::from).sum();

        if total <= 0.0 {
            return None;
        }

        let mut pick = self.unit() * total;

        for (i, w) in weights.clone().enumerate() {
            pick -= f64::from(w);

            if pick <= 0.0 {
                return Some(i);
            }
        }

        weights.count().checked_sub(1)
    }
}

mod kv {
    pub enum Kv {
        Section(Vec<(String, Kv)>),
        Str(String),
        Int(i64),
        Float(f32),
    }

    impl Kv {
        pub fn get(&self, key: &str) -> Option<&Kv> {
            self.children()
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case(key))
                .map(|(_, v)| v)
        }

        pub fn children(&self) -> &[(String, Kv)] {
            match self {
                Kv::Section(children) => children,
                _ => &[],
            }
        }

        pub fn first_section(&self) -> Option<&Kv> {
            self.children()
                .iter()
                .map(|(_, v)| v)
                .find(|v| matches!(v, Kv::Section(_)))
        }

        pub fn as_text(&self) -> String {
            match self {
                Kv::Str(s) => s.clone(),
                Kv::Int(i) => i.to_string(),
                Kv::Float(f) => f.to_string(),
                Kv::Section(_) => String::new(),
            }
        }

        pub fn as_u32(&self) -> Option<u32> {
            match self {
                Kv::Int(i) => u32::try_from(*i).ok(),
                Kv::Str(s) => s.trim().parse().ok(),
                _ => None,
            }
        }
    }

    pub fn parse(data: &[u8]) -> Option<Kv> {
        let mut pos = 0;
        let mut root = Vec::new();
        section(data, &mut pos, &mut root, 0)?;
        Some(Kv::Section(root))
    }

    fn section(data: &[u8], pos: &mut usize, out: &mut Vec<(String, Kv)>, depth: u8) -> Option<()> {
        if depth > 32 {
            return None;
        }

        while *pos < data.len() {
            let kind = data[*pos];
            *pos += 1;

            if matches!(kind, 8 | 11) {
                return Some(());
            }

            let key = cstr(data, pos)?;

            let value = match kind {
                0 => {
                    let mut children = Vec::new();
                    section(data, pos, &mut children, depth + 1)?;
                    Kv::Section(children)
                }
                1 => Kv::Str(cstr(data, pos)?),
                2 | 4 | 6 => Kv::Int(i64::from(i32::from_le_bytes(take(data, pos)?))),
                3 => Kv::Float(f32::from_le_bytes(take(data, pos)?)),
                5 => Kv::Str(wide_str(data, pos)?),
                7 => Kv::Int(u64::from_le_bytes(take(data, pos)?) as i64),
                10 => Kv::Int(i64::from_le_bytes(take(data, pos)?)),
                _ => return None,
            };

            out.push((key, value));
        }

        Some(())
    }

    fn take<const N: usize>(data: &[u8], pos: &mut usize) -> Option<[u8; N]> {
        let bytes = data.get(*pos..*pos + N)?.try_into().ok()?;
        *pos += N;
        Some(bytes)
    }

    fn cstr(data: &[u8], pos: &mut usize) -> Option<String> {
        let end = *pos + data.get(*pos..)?.iter().position(|&b| b == 0)?;
        let text = String::from_utf8_lossy(&data[*pos..end]).into_owned();
        *pos = end + 1;
        Some(text)
    }

    fn wide_str(data: &[u8], pos: &mut usize) -> Option<String> {
        let mut units = Vec::new();

        loop {
            let unit = u16::from_le_bytes(take(data, pos)?);

            if unit == 0 {
                return Some(String::from_utf16_lossy(&units));
            }

            units.push(unit);
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn parses_nested_schema() {
            let mut d = Vec::new();
            let sect = |d: &mut Vec<u8>, k: &str| {
                d.push(0);
                d.extend(k.as_bytes());
                d.push(0);
            };
            let s = |d: &mut Vec<u8>, k: &str, v: &str| {
                d.push(1);
                d.extend(k.as_bytes());
                d.push(0);
                d.extend(v.as_bytes());
                d.push(0);
            };
            let i = |d: &mut Vec<u8>, k: &str, v: i32| {
                d.push(2);
                d.extend(k.as_bytes());
                d.push(0);
                d.extend(v.to_le_bytes());
            };
            sect(&mut d, "620");
            sect(&mut d, "stats");
            sect(&mut d, "1");
            i(&mut d, "type", 4);
            sect(&mut d, "bits");
            sect(&mut d, "0");
            s(&mut d, "name", "ACH.A");
            i(&mut d, "bit", 0);
            d.extend([8, 8, 8, 8, 8, 8]);

            let root = parse(&d).unwrap();
            let defs = crate::achievements::achievements_in(&root);
            assert_eq!(defs.len(), 1);
            assert_eq!(defs[0].api_name, "ACH.A");
            assert_eq!(defs[0].stat_id, 1);
            assert_eq!(defs[0].bit, 0);
        }
    }
}
