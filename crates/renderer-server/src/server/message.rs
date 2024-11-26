use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entity::player::PlayerEntityInput,
    world::{EntityStates, TickOutput},
};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct VersionData {
    version_code: (u16, u16, u16),
    version_string: String,
}

impl VersionData {
    pub fn current() -> Self {
        const VERSION: &str = env!("CARGO_PKG_VERSION");
        const VERSION_MAJOR: &str = env!("CARGO_PKG_VERSION_MAJOR");
        const VERSION_MINOR: &str = env!("CARGO_PKG_VERSION_MINOR");
        const VERSION_PATCH: &str = env!("CARGO_PKG_VERSION_PATCH");
        Self {
            version_code: (
                VERSION_MAJOR.parse().unwrap(),
                VERSION_MINOR.parse().unwrap(),
                VERSION_PATCH.parse().unwrap(),
            ),
            version_string: String::from(VERSION),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerMessage {
    Handshake {
        version: VersionData,
    },
    SyncWorld {
        player_id: Uuid,
        entity_states: EntityStates,
    },
    TickOutput(TickOutput),
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ClientMessage {
    Handshake { version: VersionData },
    PlayerInput(PlayerEntityInput),
}
