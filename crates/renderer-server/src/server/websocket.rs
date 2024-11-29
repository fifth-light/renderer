use std::{
    error::Error,
    fmt::{self, Debug, Display, Formatter},
    io,
    marker::PhantomData,
    net::SocketAddr,
    sync::Arc,
};

use bytes::{Bytes, BytesMut};
use futures::SinkExt;
use futures::StreamExt;
use log::{info, warn};
use renderer_protocol::message::{ClientMessage, ServerMessage};
use tokio::net::TcpListener;
use tokio_serde::{Deserializer, Framed, Serializer};
use tokio_tungstenite::tungstenite::{self, Message};

use super::{serve::serve, Server};

#[derive(Debug)]
pub enum WebSocketServerError<SE> {
    WebSocket(tungstenite::Error),
    Serialize(SE),
}

impl<SE: Display> Display for WebSocketServerError<SE> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            WebSocketServerError::WebSocket(error) => Display::fmt(error, f),
            WebSocketServerError::Serialize(error) => Display::fmt(error, f),
        }
    }
}

impl<SE: Error> Error for WebSocketServerError<SE> {}

impl<SE> From<SE> for WebSocketServerError<SE> {
    fn from(value: SE) -> Self {
        Self::Serialize(value)
    }
}

#[derive(Debug)]
pub struct WebSocketServer<Codec> {
    listen_addr: SocketAddr,
    _markor: PhantomData<Codec>,
}

impl<Codec> WebSocketServer<Codec> {
    pub fn new(listen_addr: SocketAddr) -> Self {
        Self {
            listen_addr,
            _markor: PhantomData,
        }
    }
}

impl<SE, Codec> WebSocketServer<Codec>
where
    SE: Error + Send + Sync + 'static,
    Codec: Deserializer<ClientMessage, Error = SE>
        + Serializer<ServerMessage, Error = SE>
        + Send
        + Sync
        + 'static,
{
    pub async fn serve(
        &self,
        server: Arc<Server>,
        codec_factory: impl Fn() -> Codec + Send + Sync + 'static,
    ) -> io::Result<()> {
        let listener = TcpListener::bind(self.listen_addr).await?;
        let codec_factory = Arc::new(codec_factory);
        loop {
            let (stream, address) = listener.accept().await?;
            info!("Connection from {}", address);
            let server = server.clone();
            let codec_factory = codec_factory.clone();
            tokio::spawn(async move {
                let serve = serve(|| async move {
                    let stream = tokio_tungstenite::accept_async(stream)
                        .await
                        .map_err(WebSocketServerError::WebSocket)?;
                    let stream = stream
                        .filter_map::<_, Result<BytesMut, WebSocketServerError<SE>>, _>(
                            |data| async {
                                let data = match data {
                                    Ok(data) => data,
                                    Err(err) => {
                                        return Some(Err(WebSocketServerError::WebSocket(err)))
                                    }
                                };
                                match data {
                                    Message::Binary(vec) => {
                                        Some(Ok(BytesMut::from(vec.as_slice())))
                                    }
                                    Message::Ping(_) | Message::Pong(_) | Message::Close(_) => None,
                                    Message::Text(text) => Some(Ok(BytesMut::from(text.as_str()))),
                                    Message::Frame(_) => unreachable!(),
                                }
                            },
                        )
                        .sink_map_err(WebSocketServerError::WebSocket)
                        .with::<Bytes, _, _, WebSocketServerError<SE>>(|message| async move {
                            let message = Message::binary(message);
                            Ok(message)
                        });
                    let framed = Framed::new(stream, codec_factory());
                    Ok::<_, WebSocketServerError<SE>>(framed)
                });
                match server.serve(serve).await {
                    Ok(_) => {
                        info!("Connection closed from {}", address);
                    }
                    Err(err) => {
                        warn!("Serve failed: {:?}", err);
                    }
                }
            });
        }
    }
}
