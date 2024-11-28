use std::{error::Error, fmt::Debug, net::SocketAddr, pin::Pin, sync::Arc};

use renderer_protocol::message::{ClientMessage, ServerMessage};

#[cfg(feature = "tokio-transport")]
pub mod tokio;

#[derive(Debug)]
pub enum TransportState {
    Connecting,
    Connected,
    Closed,
    Failed(Box<dyn Error>),
}

pub trait TransportParam {
    fn connect(&self) -> Box<dyn Transport>;
}

pub trait Transport: Debug {
    fn state(&self) -> TransportState;
    fn receive(&mut self) -> Result<Option<ServerMessage>, Box<dyn Error>>;
    fn send(&mut self, message: ClientMessage) -> Result<(), Box<dyn Error>>;
    fn close(self);
}
