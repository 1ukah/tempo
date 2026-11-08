#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Status {
    Online,
    Away,
    Busy,
    Snooze,
    LookingToTrade,
    LookingToPlay,
    Invisible,
    Offline,
}

pub const DEFAULT: Status = Status::Online;

const NAMES: &[(Status, &[&str])] = &[
    (Status::Online, &["online"]),
    (Status::Away, &["away"]),
    (Status::Busy, &["busy", "dnd"]),
    (Status::Snooze, &["snooze"]),
    (Status::LookingToTrade, &["trade", "looking-to-trade"]),
    (Status::LookingToPlay, &["play", "looking-to-play"]),
    (Status::Invisible, &["invisible"]),
    (Status::Offline, &["offline"]),
];

impl Status {
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim().to_ascii_lowercase();

        NAMES
            .iter()
            .find(|(_, names)| names.contains(&text.as_str()))
            .map(|&(status, _)| status)
    }

    pub fn name(self) -> &'static str {
        NAMES
            .iter()
            .find(|(status, _)| *status == self)
            .map_or("online", |(_, names)| names[0])
    }

    pub fn all_names() -> String {
        NAMES
            .iter()
            .map(|(_, names)| names[0])
            .collect::<Vec<_>>()
            .join(", ")
    }

    pub fn persona_state(self) -> u32 {
        match self {
            Status::Offline => 0,
            Status::Online => 1,
            Status::Busy => 2,
            Status::Away => 3,
            Status::Snooze => 4,
            Status::LookingToTrade => 5,
            Status::LookingToPlay => 6,
            Status::Invisible => 7,
        }
    }
}
