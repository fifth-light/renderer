use std::{
    error::Error,
    fmt::{self, Display, Formatter},
};

use log::{info, warn};
use renderer_protocol::{
    message::{ClientMessage, ServerMessage},
    version::VersionData,
};
use uuid::Uuid;
use world::World;

use crate::{
    gui::{
        connect::{ConnectParam, ConnectionStatus},
        GuiState,
    },
    renderer::Renderer,
    transport::{Transport, TransportState},
};

pub mod entity;
pub mod world;

#[derive(Debug)]
enum ConnectionError {
    Handshake,
    WorldSync,
    Message,
}

impl Display for ConnectionError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ConnectionError::Handshake => write!(f, "Bad handshake"),
            ConnectionError::WorldSync => write!(f, "Bad world sync message"),
            ConnectionError::Message => writeln!(f, "Bad message"),
        }
    }
}

impl Error for ConnectionError {}

#[derive(Debug, Default)]
pub enum ConnectionState {
    #[default]
    SendClientHandshake,
    WaitingServerHandshake,
    WaitingWorldSync {
        server_version: VersionData,
    },
    Connected {
        server_version: VersionData,
        player_id: Uuid,
        world: World,
    },
}

impl ConnectionState {
    fn tick(
        &mut self,
        transport: &mut dyn Transport,
        renderer: &mut Renderer,
    ) -> Result<bool, Box<dyn Error>> {
        match self {
            ConnectionState::SendClientHandshake => {
                info!("Handshake sent");
                transport.send(ClientMessage::Handshake {
                    version: VersionData::current(),
                })?;

                *self = ConnectionState::WaitingServerHandshake;
                Ok(true)
            }
            ConnectionState::WaitingServerHandshake => {
                let Some(message) = transport.receive()? else {
                    return Ok(true);
                };

                let ServerMessage::Handshake { version } = message else {
                    return Err(Box::new(ConnectionError::Handshake));
                };
                info!("Server handshake received");

                *self = ConnectionState::WaitingWorldSync {
                    server_version: version,
                };
                Ok(true)
            }
            ConnectionState::WaitingWorldSync { server_version } => {
                let Some(message) = transport.receive()? else {
                    return Ok(true);
                };

                let ServerMessage::SyncWorld {
                    player_id,
                    entity_states,
                } = message
                else {
                    return Err(Box::new(ConnectionError::WorldSync));
                };

                info!("Server world sync received with player id {:?}", player_id);

                let world = World::new(entity_states);

                *self = ConnectionState::Connected {
                    server_version: server_version.clone(),
                    player_id,
                    world,
                };
                Ok(true)
            }
            ConnectionState::Connected {
                world, player_id, ..
            } => {
                if let Some(player) = world.entities.player.get_mut(player_id) {
                    player.update(renderer.camera());

                    let mut input = vec![];
                    player.send_input(&mut input);
                    transport.send(ClientMessage::PlayerInput(input))?;
                } else {
                    warn!("Player not found: {:?}", player_id);
                }

                while let Some(message) = transport.receive()? {
                    match message {
                        ServerMessage::Handshake { .. } | ServerMessage::SyncWorld { .. } => {
                            return Err(Box::new(ConnectionError::Message));
                        }
                        ServerMessage::TickOutput(tick_output) => {
                            info!("Tick output: {:?}", tick_output);
                            world.update(tick_output);
                        }
                    }
                }
                Ok(true)
            }
        }
    }

    fn world(&self) -> Option<&World> {
        match self {
            ConnectionState::Connected { world, .. } => Some(world),
            _ => None,
        }
    }
}

#[derive(Debug)]
enum ClientState {
    Connecting,
    Connected(ConnectionState),
    Closed,
}

#[derive(Debug)]
pub struct Client {
    state: ClientState,
    transport: Box<dyn Transport>,
}

impl Client {
    pub fn tick<CP: ConnectParam>(
        &mut self,
        renderer: &mut Renderer,
        gui_state: &mut GuiState<CP>,
    ) -> bool {
        match self.transport.state() {
            TransportState::Connecting => {
                self.state = ClientState::Connecting;
                true
            }
            TransportState::Connected => {
                if let ClientState::Connected(ref mut state) = self.state {
                    let result = state.tick(self.transport.as_mut(), renderer);
                    match result {
                        Ok(result) => result,
                        Err(error) => {
                            gui_state.add_error(error.to_string());
                            false
                        }
                    }
                } else {
                    let mut state = ConnectionState::default();
                    let result = state.tick(self.transport.as_mut(), renderer);
                    self.state = ClientState::Connected(state);
                    match result {
                        Ok(result) => result,
                        Err(error) => {
                            gui_state.add_error(error.to_string());
                            false
                        }
                    }
                }
            }
            TransportState::Closed => {
                self.state = ClientState::Closed;
                false
            }
            TransportState::Failed(error) => {
                gui_state.add_error(error.to_string());
                false
            }
        }
    }

    pub fn world(&self) -> Option<&World> {
        if let ClientState::Connected(ref state) = self.state {
            state.world()
        } else {
            None
        }
    }

    pub fn connection_status(&self) -> ConnectionStatus {
        match &self.state {
            ClientState::Connecting => ConnectionStatus::Connecting,
            ClientState::Connected(state) => match state {
                ConnectionState::SendClientHandshake | ConnectionState::WaitingServerHandshake => {
                    ConnectionStatus::Handshaking
                }
                ConnectionState::WaitingWorldSync { server_version } => {
                    ConnectionStatus::SyncingWorld {
                        server_version: server_version.clone(),
                    }
                }
                ConnectionState::Connected { .. } => ConnectionStatus::Connected,
            },
            ClientState::Closed => ConnectionStatus::Closed,
        }
    }

    pub fn new(transport: Box<dyn Transport>) -> Self {
        Self {
            state: ClientState::Connecting,
            transport,
        }
    }
}
