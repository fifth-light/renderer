use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entity::EntityStates, input::PlayerEntityInput, tick::TickOutput, version::VersionData,
};

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
