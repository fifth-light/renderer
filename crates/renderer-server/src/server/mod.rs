use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    time::Duration,
};

use connection::{Connection, ConnectionError};
use futures::SinkExt;
use serde::{Deserialize, Serialize};
use serve::Serve;
use tokio::{
    sync::{Mutex, RwLock},
    time::sleep,
};

use crate::world::World;

pub mod connection;
pub mod message;
pub mod serve;
pub mod websocket;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub handshake_timeout: Duration,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            handshake_timeout: Duration::from_secs(10),
        }
    }
}

#[derive(Debug)]
pub struct Server {
    run_lock: Mutex<()>,
    pub config: ServerConfig,
    pub world: RwLock<World>,
}

pub enum ServeError<S: Serve> {
    Connect(S::ConnectError),
    Connection(ConnectionError<S::SendError, S::RecvError>),
    Close(S::SendError),
}

impl<S: Serve> Debug for ServeError<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Self::Connect(err) => f.debug_tuple("Connect").field(err).finish(),
            Self::Connection(err) => f.debug_tuple("Connection").field(err).finish(),
            Self::Close(err) => f.debug_tuple("Close").field(err).finish(),
        }
    }
}

impl<S: Serve> Display for ServeError<S> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            ServeError::Connect(err) => write!(f, "Failed to connect: {:?}", err),
            ServeError::Connection(err) => write!(f, "Connection failed: {:?}", err),
            ServeError::Close(err) => write!(f, "Failed to close: {:?}", err),
        }
    }
}

impl<S: Serve> Error for ServeError<S> {}

impl Server {
    pub fn new(config: ServerConfig) -> Self {
        Self {
            run_lock: Mutex::new(()),
            config,
            world: RwLock::new(World::empty()),
        }
    }

    pub async fn run(&self) -> ! {
        let _lock = self.run_lock.lock().await;
        loop {
            sleep(Duration::from_millis(50)).await;
        }
    }

    pub async fn serve<S: Serve>(&self, serve: S) -> Result<(), ServeError<S>> {
        let transport = serve.serve().await.map_err(ServeError::Connect)?;
        let connection = Connection::new(transport, self);
        match connection.run().await {
            Ok(mut transport) => {
                transport.close().await.map_err(ServeError::Close)?;
                Ok(())
            }
            Err(error) => Err(ServeError::Connection(error)),
        }
    }
}

impl Default for Server {
    fn default() -> Self {
        Self::new(ServerConfig::default())
    }
}
