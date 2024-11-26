use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    pin::Pin,
    time::Duration,
};

use futures::{Sink, SinkExt, Stream, StreamExt, TryStreamExt};
use glam::Vec3;
use log::{info, trace};
use tokio::{select, sync::mpsc, time::sleep};
use uuid::Uuid;

use crate::entity::{player::PlayerEntity, Entity};

use super::{
    message::{ClientMessage, ServerMessage, VersionData},
    Server,
};

#[derive(Debug)]
pub struct HandshakeData {
    pub version: VersionData,
}

#[derive(Debug)]
pub struct Connection<'server, T, SE, RE>
where
    T: Stream<Item = Result<ClientMessage, RE>>
        + Sink<ServerMessage, Error = SE>
        + Send
        + Sync
        + 'static,
    SE: Error + Send + Sync,
    RE: Error + Send + Sync,
{
    transport: T,
    server: &'server Server,
}

#[derive(Debug)]
pub enum ConnectionError<SE, RE>
where
    SE: Error + Send + Sync,
    RE: Error + Send + Sync,
{
    SendError(SE),
    ReceiveError(RE),
    NoHandshake,
    PlayerAlreadyExists,
    OutputChannelAlreadyExists,
    OutputChannelDestroyed,
    BadMessage(ClientMessage),
    HandshakeTimeout(Duration),
}

impl<SE, RE> Display for ConnectionError<SE, RE>
where
    SE: Error + Send + Sync,
    RE: Error + Send + Sync,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::SendError(err) => write!(f, "Failed to send: {:?}", err),
            Self::ReceiveError(err) => write!(f, "Failed to receive: {:?}", err),
            Self::NoHandshake => write!(f, "No handshake received"),
            Self::PlayerAlreadyExists => write!(f, "Player already exists"),
            Self::OutputChannelAlreadyExists => write!(f, "Output channel already exists"),
            Self::OutputChannelDestroyed => write!(f, "Output channel is destroyed"),
            Self::BadMessage(message) => write!(f, "Bad message from client: {:?}", message),
            Self::HandshakeTimeout(duration) => {
                write!(f, "Handshake timeout after {} ms", duration.as_millis())
            }
        }
    }
}

impl<SE, RE> Error for ConnectionError<SE, RE>
where
    SE: Error + Send + Sync,
    RE: Error + Send + Sync,
{
}

impl<'server, T, SE, RE> Connection<'server, T, SE, RE>
where
    T: Stream<Item = Result<ClientMessage, RE>>
        + Sink<ServerMessage, Error = SE>
        + Send
        + Sync
        + 'static,
    SE: Error + Send + Sync,
    RE: Error + Send + Sync,
{
    pub fn new(transport: T, server: &'server Server) -> Self {
        Self { transport, server }
    }

    pub async fn run(self) -> Result<Pin<Box<T>>, ConnectionError<SE, RE>> {
        let mut transport = Box::pin(self.transport);

        // Send initial handshake
        transport
            .send(ServerMessage::Handshake {
                version: VersionData::current(),
            })
            .await
            .map_err(ConnectionError::SendError)?;

        // Receive client handshake
        let handshake_timeout = self.server.config.handshake_timeout;
        let message = select! {
            message = transport.next() => {
                if let Some(message) = message {
                    message
                } else {
                    return Err(ConnectionError::NoHandshake)
                }
            }
            _ = sleep(handshake_timeout) => {
                return Err(ConnectionError::HandshakeTimeout(handshake_timeout));
            }
        };
        let message = message.map_err(ConnectionError::ReceiveError)?;
        let client_version = match message {
            ClientMessage::Handshake { version } => version,
            _ => return Err(ConnectionError::BadMessage(message)),
        };
        info!("Client version: {:?}", client_version);

        // Lock server state
        let mut state = self.server.state.write().await;

        // Add player to world
        let player_id = Uuid::new_v4();

        // Add output channel
        let (output_tx, mut output_rx) = mpsc::unbounded_channel();
        if state.insert_channel(player_id, output_tx).is_err() {
            return Err(ConnectionError::OutputChannelAlreadyExists);
        }

        let player = PlayerEntity::new(player_id, Vec3::ZERO);
        info!(
            "Player {} logged in at {:?}",
            player.id(),
            player.position()
        );
        if state.world.insert_player(player).is_err() {
            state.remove_channel(player_id);
            return Err(ConnectionError::PlayerAlreadyExists);
        }

        // Copy entity state, and send them to client
        let entity_states = state.world.entities.state();

        drop(state);

        let run_result = async move {
            transport
                .send(ServerMessage::SyncWorld {
                    player_id,
                    entity_states,
                })
                .await
                .map_err(ConnectionError::SendError)?;

            // Handle input and output
            loop {
                tokio::select! {
                    message = transport.try_next() => {
                        let message = message.map_err(ConnectionError::ReceiveError)?;
                        let Some(message) = message else { break };
                        match message {
                            ClientMessage::Handshake { .. } => {
                                return Err(ConnectionError::BadMessage(message))
                            }
                            ClientMessage::PlayerInput(input) => {
                                trace!("Entity input: {:?}", input);
                                self.server.input_queue.push((player_id, input));
                            }
                        }
                    }
                    output = output_rx.recv() => {
                        let Some(output) = output else {
                            return Err(ConnectionError::OutputChannelDestroyed);
                        };
                        transport
                            .send(ServerMessage::TickOutput((*output).clone()))
                            .await
                            .map_err(ConnectionError::SendError)?;
                    }
                }
            }

            Ok(transport)
        }
        .await;

        // Queue removal of player
        let mut state = self.server.state.write().await;
        state.world.entities.queue_remove_player(player_id);
        // Remove output queue
        state.output_queue.remove(&player_id);

        run_result
    }
}
