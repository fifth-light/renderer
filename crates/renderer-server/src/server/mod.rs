use std::{
    collections::{hash_map::Entry, HashMap},
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    sync::Arc,
    time::Duration,
};

use connection::{Connection, ConnectionError};
use crossbeam::queue::SegQueue;
use futures::SinkExt;
use log::{trace, warn};
use renderer_perf_tracker::PerformanceTracker;
use serde::{Deserialize, Serialize};
use serve::Serve;
use tokio::{
    sync::{mpsc, Mutex, RwLock},
    time::sleep,
};
use uuid::Uuid;
use web_time::Instant;

use crate::{
    entity::player::PlayerEntityInput,
    world::{TickOutput, World},
};

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

// Number from a legendary game
const TARGET_TICK_RATE: usize = 20;

#[derive(Debug, Default)]
pub struct ServerState {
    pub world: World,
    output_queue: HashMap<Uuid, mpsc::UnboundedSender<Arc<TickOutput>>>,
}

#[derive(Debug)]
pub struct ChannelAlreadyExists;

impl Display for ChannelAlreadyExists {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Channel already exists")
    }
}

impl Error for ChannelAlreadyExists {}

impl ServerState {
    pub fn insert_channel(
        &mut self,
        id: Uuid,
        channel: mpsc::UnboundedSender<Arc<TickOutput>>,
    ) -> Result<(), ChannelAlreadyExists> {
        match self.output_queue.entry(id) {
            Entry::Occupied(_) => Err(ChannelAlreadyExists),
            Entry::Vacant(entry) => {
                entry.insert(channel);
                Ok(())
            }
        }
    }

    pub fn remove_channel(&mut self, id: Uuid) -> Option<mpsc::UnboundedSender<Arc<TickOutput>>> {
        self.output_queue.remove(&id)
    }
}

#[derive(Debug, Default)]
pub struct Server {
    run_lock: Mutex<()>,
    pub input_queue: SegQueue<(Uuid, PlayerEntityInput)>,
    pub config: ServerConfig,
    pub state: RwLock<ServerState>,
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
            input_queue: SegQueue::new(),
            config,
            state: RwLock::new(ServerState::default()),
        }
    }

    pub async fn run(&self) -> ! {
        let _lock = self.run_lock.lock().await;
        let mut performance_tracker = PerformanceTracker::new(TARGET_TICK_RATE);
        let target_frame_time = Duration::from_secs(1) / TARGET_TICK_RATE as u32;
        loop {
            let start_time = Instant::now();

            let mut state = self.state.write().await;
            while let Some((id, input)) = self.input_queue.pop() {
                state.world.entities.process_player_inputs(id, input);
            }

            let output = state.world.tick();
            trace!("Tick output: {:?}", output);
            let output = Arc::new(output);
            state.output_queue.retain(|id, channel| {
                let result = channel.send(output.clone());
                if result.is_err() {
                    warn!("Output channel for id {:?} was closed", id);
                }
                result.is_ok()
            });

            drop(state);

            let end_time = Instant::now();
            let frame_time = end_time - start_time;
            performance_tracker.add_sample(frame_time, end_time);

            if let Some(avg_tick_time) = performance_tracker.avg_frame_time() {
                let sleep_time = target_frame_time - avg_tick_time;
                sleep(sleep_time).await;
            } else {
                unreachable!("Missing average tick time data");
            }

            trace!("TPS: {:?}", performance_tracker.fps());
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
