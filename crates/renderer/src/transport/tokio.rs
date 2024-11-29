use std::{
    error::Error as StdError,
    fmt::{self, Debug, Display, Formatter},
    io,
    pin::pin,
    sync::Arc,
    thread::{self, JoinHandle},
};

use bytes::{Bytes, BytesMut};
use futures::{SinkExt, StreamExt};
use log::warn;
use renderer_protocol::message::{ClientMessage, ServerMessage};
use tokio::{
    runtime::Runtime,
    select,
    sync::{mpsc, oneshot, Mutex},
};
use tokio_serde::{Deserializer, Framed, Serializer};
use tokio_tungstenite::tungstenite::{self, http::Request, Message};

use super::{Transport, TransportParam, TransportState};

#[derive(Debug)]
pub enum Error {
    NotConnected,
    Closed,
    ConnectFailed(Arc<io::Error>),
    WebsocketFailed(Arc<tungstenite::Error>),
    Serialize(Arc<dyn StdError>),
    Send(Arc<dyn StdError>),
    Receive(Arc<dyn StdError>),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotConnected => write!(f, "Not connected to the server"),
            Error::Closed => write!(f, "Connection closed"),
            Error::ConnectFailed(error) => Display::fmt(error, f),
            Error::WebsocketFailed(error) => Display::fmt(error, f),
            Error::Serialize(error) => Display::fmt(error, f),
            Error::Send(error) => Display::fmt(error, f),
            Error::Receive(error) => Display::fmt(error, f),
        }
    }
}

impl StdError for Error {}

#[derive(Default, Debug)]
enum State {
    #[default]
    Connecting,
    ConnectFailed(Arc<io::Error>),
    ConnectWebsocketFailed(Arc<tungstenite::Error>),
    Connected {
        send_tx: mpsc::UnboundedSender<ClientMessage>,
        receive_rx: mpsc::UnboundedReceiver<ServerMessage>,
    },
    ReceiveFailed(Arc<dyn StdError + Send + Sync + 'static>),
    SendFailed(Arc<dyn StdError + Send + Sync + 'static>),
    ConnectionClosed,
}

#[derive(Debug)]
pub struct TokioTransport {
    thread_handle: Option<JoinHandle<()>>,
    cancel_tx: Option<oneshot::Sender<()>>,
    close_tx: Option<oneshot::Sender<()>>,
    state: Arc<Mutex<State>>,
}

#[derive(Debug)]
pub enum TransportError<SE> {
    WebSocket(tungstenite::Error),
    Serialize(SE),
}

impl<SE: Display> Display for TransportError<SE> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            TransportError::WebSocket(error) => Display::fmt(error, f),
            TransportError::Serialize(error) => Display::fmt(error, f),
        }
    }
}

impl<SE: StdError> StdError for TransportError<SE> {}

impl<SE> From<SE> for TransportError<SE> {
    fn from(value: SE) -> Self {
        Self::Serialize(value)
    }
}

fn transport_thread<SE, Codec>(
    request: Request<()>,
    mut cancel_rx: oneshot::Receiver<()>,
    mut close_rx: oneshot::Receiver<()>,
    state: Arc<Mutex<State>>,
    codec: Codec,
) where
    SE: StdError + Send + Sync + 'static,
    Codec: Deserializer<ServerMessage, Error = SE>
        + Serializer<ClientMessage, Error = SE>
        + Send
        + Sync
        + 'static,
{
    let runtime = match Runtime::new() {
        Ok(runtime) => runtime,
        Err(err) => {
            let mut state = state.blocking_lock();
            warn!("Failed to create runtime: {:?}", err);
            let err = Arc::new(err);
            *state = State::ConnectFailed(err.clone());
            return;
        }
    };

    let result = runtime.block_on(async {
        let stream = select! {
            biased;
            _ = &mut cancel_rx => { return Ok(()); }
            _ = &mut close_rx => { return Ok(()); }
            stream = tokio_tungstenite::connect_async(request) => stream
        };
        let (stream, _response) = match stream {
            Ok(stream) => stream,
            Err(err) => {
                let mut state = state.lock().await;
                *state = State::ConnectWebsocketFailed(Arc::new(err));
                return Err(());
            }
        };

        let (send_tx, mut send_rx) = mpsc::unbounded_channel();
        let (receive_tx, receive_rx) = mpsc::unbounded_channel();
        {
            let mut state = state.lock().await;
            *state = State::Connected {
                send_tx,
                receive_rx,
            };
            drop(state);
        }

        let stream = stream
            .filter_map::<_, Result<BytesMut, TransportError<SE>>, _>(|data| async {
                let data = match data {
                    Ok(data) => data,
                    Err(err) => return Some(Err(TransportError::WebSocket(err))),
                };
                match data {
                    Message::Binary(vec) => Some(Ok(BytesMut::from(vec.as_slice()))),
                    Message::Ping(_) | Message::Pong(_) | Message::Close(_) => None,
                    Message::Text(text) => Some(Ok(BytesMut::from(text.as_str()))),
                    Message::Frame(_) => unreachable!(),
                }
            })
            .sink_map_err(TransportError::WebSocket)
            .with::<Bytes, _, _, TransportError<SE>>(|message| async move {
                let message = Message::binary(message);
                Ok(message)
            });
        let framed = Framed::new(stream, codec);
        let mut transport = pin!(framed);
        loop {
            select! {
                _ = &mut cancel_rx => { return Ok(()); }
                _ = &mut close_rx => { return Ok(()); }
                message = transport.next() => {
                    let Some(message) = message else { return Ok(()) };
                    let message = match message {
                        Ok(message) => message,
                        Err(err) => {
                            let mut state = state.lock().await;
                            *state = State::ReceiveFailed(Arc::new(err));
                            return Err(());
                        },
                    };
                    if receive_tx.send(message).is_err() {
                        return Ok(());
                    }
                }
                message = send_rx.recv() => {
                    let Some(message) = message else { return Ok(()) };
                    if let Err(err) = transport.send(message).await {
                        let mut state = state.lock().await;
                        *state = State::SendFailed(Arc::new(err));
                        return Err(());
                    }
                }
            }
        }
    });

    if result.is_ok() {
        let mut state = state.blocking_lock();
        *state = State::ConnectionClosed;
    }

    runtime.shutdown_background();
}

impl Transport for TokioTransport {
    fn state(&self) -> TransportState {
        let state = self.state.blocking_lock();
        match &*state {
            State::Connecting => TransportState::Connecting,
            State::ConnectFailed(err) => {
                TransportState::Failed(Box::new(Error::ConnectFailed(err.clone())))
            }
            State::ConnectWebsocketFailed(err) => {
                TransportState::Failed(Box::new(Error::WebsocketFailed(err.clone())))
            }
            State::ReceiveFailed(err) => {
                TransportState::Failed(Box::new(Error::Receive(err.clone())))
            }
            State::SendFailed(err) => TransportState::Failed(Box::new(Error::Send(err.clone()))),
            State::ConnectionClosed => TransportState::Closed,
            State::Connected { .. } => TransportState::Connected,
        }
    }

    fn receive(&mut self) -> Result<Option<ServerMessage>, Box<dyn StdError>> {
        let mut state = self.state.blocking_lock();
        match &mut *state {
            State::Connecting => Err(Error::NotConnected),
            State::ConnectFailed(err) => Err(Error::ConnectFailed(err.clone())),
            State::ConnectWebsocketFailed(err) => Err(Error::WebsocketFailed(err.clone())),
            State::ReceiveFailed(err) => Err(Error::Receive(err.clone())),
            State::SendFailed(err) => Err(Error::Send(err.clone())),
            State::ConnectionClosed => Err(Error::Closed),
            State::Connected { receive_rx, .. } => {
                if let Ok(message) = receive_rx.try_recv() {
                    Ok(Some(message))
                } else {
                    Ok(None)
                }
            }
        }
        .map_err(|err| {
            let err: Box<dyn StdError> = Box::new(err);
            err
        })
    }

    fn send(&mut self, message: ClientMessage) -> Result<(), Box<dyn StdError>> {
        let mut state = self.state.blocking_lock();
        match &mut *state {
            State::Connecting => Err(Error::NotConnected),
            State::ConnectFailed(err) => Err(Error::ConnectFailed(err.clone())),
            State::ConnectWebsocketFailed(err) => Err(Error::WebsocketFailed(err.clone())),
            State::ReceiveFailed(err) => Err(Error::Receive(err.clone())),
            State::SendFailed(err) => Err(Error::Send(err.clone())),
            State::ConnectionClosed => Err(Error::Closed),
            State::Connected { send_tx, .. } => {
                if send_tx.send(message).is_ok() {
                    Ok(())
                } else {
                    Err(Error::Closed)
                }
            }
        }
        .map_err(|err| {
            let err: Box<dyn StdError> = Box::new(err);
            err
        })
    }

    fn close(mut self) {
        if let Some(close_tx) = self.close_tx.take() {
            let _ = close_tx.send(());
        }
        if let Some(handle) = self.thread_handle.take() {
            handle.join().unwrap();
        }
    }
}

impl Drop for TokioTransport {
    fn drop(&mut self) {
        if let Some(cancel_tx) = self.cancel_tx.take() {
            let _ = cancel_tx.send(());
        }
        if let Some(handle) = self.thread_handle.take() {
            handle.join().unwrap();
        }
    }
}

pub struct TokioTransportParam<CodecBuilder> {
    request: Request<()>,
    codec_builder: CodecBuilder,
}

impl<SE, Codec, CodecBuilder> TransportParam for TokioTransportParam<CodecBuilder>
where
    SE: StdError + Send + Sync + 'static,
    Codec: Deserializer<ServerMessage, Error = SE>
        + Serializer<ClientMessage, Error = SE>
        + Send
        + Sync
        + 'static,
    CodecBuilder: Fn() -> Codec,
{
    fn connect(&self) -> Box<dyn Transport> {
        let state = Arc::new(Mutex::new(State::default()));
        let thread_state = state.clone();
        let (cancel_tx, cancel_rx) = oneshot::channel();
        let (close_tx, close_rx) = oneshot::channel();
        let request = self.request.clone();
        let codec = (self.codec_builder)();
        let transport = TokioTransport {
            thread_handle: Some(thread::spawn(move || {
                transport_thread(request, cancel_rx, close_rx, thread_state, codec);
            })),
            cancel_tx: Some(cancel_tx),
            close_tx: Some(close_tx),
            state,
        };
        Box::new(transport)
    }
}

impl<CodecBuilder> TokioTransportParam<CodecBuilder> {
    pub fn new(request: Request<()>, codec_builder: CodecBuilder) -> Self {
        Self {
            request,
            codec_builder,
        }
    }
}
