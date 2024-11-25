use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    pin::Pin,
    time::Duration,
};

use futures::{Sink, SinkExt, Stream, StreamExt, TryStreamExt};
use log::{info, trace};
use tokio::{select, time::sleep};

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

        // Sync world to client
        let world = self.server.world.read().await;
        let entity_states = world.entity_states();
        drop(world);
        transport
            .send(ServerMessage::SyncWorld { entity_states })
            .await
            .map_err(ConnectionError::SendError)?;

        // Handle input and output
        while let Some(message) = transport
            .try_next()
            .await
            .map_err(ConnectionError::ReceiveError)?
        {
            match message {
                ClientMessage::Handshake { .. } => {
                    return Err(ConnectionError::BadMessage(message))
                }
                ClientMessage::EntityInput { id, input } => {
                    trace!("Entity input: {} -> {:?}", id, input)
                }
            }
        }

        Ok(transport)
    }
}
