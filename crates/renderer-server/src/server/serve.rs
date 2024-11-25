use std::{error::Error, future::Future};

use futures::{Sink, Stream};

use super::message::{ClientMessage, ServerMessage};

#[allow(async_fn_in_trait)]
pub trait Serve {
    type Transport: Stream<Item = Result<ClientMessage, Self::RecvError>>
        + Sink<ServerMessage, Error = Self::SendError>
        + Send
        + Sync
        + 'static;
    type ConnectError: Error + Send + Sync;
    type SendError: Error + Send + Sync;
    type RecvError: Error + Send + Sync;

    async fn serve(self) -> Result<Self::Transport, Self::ConnectError>;
}

#[derive(Debug, Clone)]
pub struct ServeFn<F, Fut, T, E, SE, RE>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    T: Stream<Item = Result<ClientMessage, RE>>
        + Sink<ServerMessage, Error = SE>
        + Send
        + Sync
        + 'static,
    E: Error + Send + Sync,
    SE: Error + Send + Sync,
    RE: Error + Send + Sync,
{
    f: F,
}

impl<F, Fut, T, E, SE, RE> Serve for ServeFn<F, Fut, T, E, SE, RE>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    T: Stream<Item = Result<ClientMessage, RE>>
        + Sink<ServerMessage, Error = SE>
        + Send
        + Sync
        + 'static,
    E: Error + Send + Sync,
    SE: Error + Send + Sync,
    RE: Error + Send + Sync,
{
    type Transport = T;
    type ConnectError = E;
    type SendError = SE;
    type RecvError = RE;

    async fn serve(self) -> Result<Self::Transport, Self::ConnectError> {
        (self.f)().await
    }
}

pub fn serve<F, Fut, T, E, SE, RE>(f: F) -> ServeFn<F, Fut, T, E, SE, RE>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, E>>,
    T: Stream<Item = Result<ClientMessage, RE>>
        + Sink<ServerMessage, Error = SE>
        + Send
        + Sync
        + 'static,
    E: Error + Send + Sync,
    SE: Error + Send + Sync,
    RE: Error + Send + Sync,
{
    ServeFn { f }
}
