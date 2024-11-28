use std::{
    cell::RefCell,
    error::Error as StdError,
    fmt::{self, Display, Formatter},
    rc::Rc,
    sync::mpsc::{self, TryRecvError},
};

use log::warn;
use renderer::{
    protocol::message::{ClientMessage, ServerMessage},
    transport::{Transport, TransportParam, TransportState},
};
use wasm_bindgen::{prelude::Closure, JsCast, JsValue};
use web_sys::{
    js_sys::{ArrayBuffer, JsString, Uint8Array},
    BinaryType, ErrorEvent, MessageEvent, Url, WebSocket,
};

use super::codec::Codec;

#[derive(Clone, Debug)]
enum Error {
    NotConnected,
    Closed,
    ConnectFailed(JsError),
    Serialize(Rc<dyn StdError>),
    Send(JsError),
    Receive(Rc<dyn StdError>),
}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Error::NotConnected => write!(f, "Not connected to the server"),
            Error::Closed => write!(f, "Connection closed"),
            Error::ConnectFailed(error) => Display::fmt(error, f),
            Error::Serialize(error) => Display::fmt(error, f),
            Error::Send(error) => Display::fmt(error, f),
            Error::Receive(error) => Display::fmt(error, f),
        }
    }
}

impl StdError for Error {}

#[derive(Debug, Clone)]
struct JsError(JsValue);

impl Display for JsError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "{}", JsString::from(self.0.clone()))
    }
}

impl StdError for JsError {}

impl From<JsValue> for JsError {
    fn from(value: JsValue) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
struct State<C>
where
    C: Codec<ServerMessage, ClientMessage>,
{
    websocket: WebSocket,
    codec: C,
    recv_rx: mpsc::Receiver<ServerMessage>,
    _on_error: Closure<dyn FnMut(ErrorEvent)>,
    _on_message: Closure<dyn FnMut(MessageEvent)>,
}

impl<C> State<C>
where
    C: Codec<ServerMessage, ClientMessage>,
{
    fn new(
        websocket: WebSocket,
        codec: C,
        recv_rx: mpsc::Receiver<ServerMessage>,
        mut error_handler: impl FnMut(JsError) + 'static,
        mut message_handler: impl FnMut(JsValue) + 'static,
    ) -> Self {
        let on_error =
            Closure::new(move |error: ErrorEvent| error_handler(JsError::from(error.error())));
        let on_message = Closure::new(move |message: MessageEvent| message_handler(message.data()));
        websocket.set_onerror(Some(on_error.as_ref().unchecked_ref()));
        websocket.set_binary_type(BinaryType::Arraybuffer);
        Self {
            websocket,
            codec,
            recv_rx,
            _on_error: on_error,
            _on_message: on_message,
        }
    }
}

impl<C> Drop for State<C>
where
    C: Codec<ServerMessage, ClientMessage>,
{
    fn drop(&mut self) {
        let _ = self.websocket.close();
        self.websocket.set_onerror(None);
        self.websocket.set_onmessage(None);
    }
}

#[derive(Debug)]
struct WebSocketTransport<C>
where
    C: Codec<ServerMessage, ClientMessage>,
{
    state: Rc<RefCell<Option<State<C>>>>,
    error: Rc<RefCell<Option<Error>>>,
}

impl<C> Transport for WebSocketTransport<C>
where
    C: Codec<ServerMessage, ClientMessage>,
{
    fn state(&self) -> TransportState {
        if let Some(State { websocket, .. }) = self.state.borrow().as_ref() {
            match websocket.ready_state() {
                WebSocket::CONNECTING => TransportState::Connecting,
                WebSocket::CLOSED | WebSocket::CLOSING => TransportState::Closed,
                WebSocket::OPEN => TransportState::Connected,
                _ => unreachable!("Bad ready state"),
            }
        } else if let Some(error) = self.error.borrow().as_ref() {
            TransportState::Failed(Box::new(error.clone()))
        } else {
            unreachable!("Bad transport state")
        }
    }

    fn receive(&mut self) -> Result<Option<ServerMessage>, Box<dyn StdError>> {
        if let Some(State {
            websocket, recv_rx, ..
        }) = self.state.borrow_mut().as_ref()
        {
            match websocket.ready_state() {
                WebSocket::CONNECTING => Err(Box::new(Error::NotConnected)),
                WebSocket::CLOSED | WebSocket::CLOSING => Err(Box::new(Error::Closed)),
                WebSocket::OPEN => match recv_rx.try_recv() {
                    Ok(message) => Ok(Some(message)),
                    Err(TryRecvError::Empty) => Ok(None),
                    Err(TryRecvError::Disconnected) => Err(Box::new(Error::Closed)),
                },
                _ => unreachable!("Bad ready state"),
            }
        } else if let Some(error) = self.error.borrow().as_ref() {
            Err(Box::new(error.clone()))
        } else {
            unreachable!("Bad transport state")
        }
    }

    fn send(&mut self, message: ClientMessage) -> Result<(), Box<dyn StdError>> {
        if let Some(State {
            websocket, codec, ..
        }) = self.state.borrow_mut().as_ref()
        {
            match websocket.ready_state() {
                WebSocket::CONNECTING => Err(Box::new(Error::NotConnected)),
                WebSocket::CLOSED | WebSocket::CLOSING => Err(Box::new(Error::Closed)),
                WebSocket::OPEN => {
                    let data = codec
                        .serialize(&message)
                        .map_err(|err| Box::new(Error::Serialize(err)))?;
                    Ok(websocket
                        .send_with_u8_array(data.as_slice())
                        .map_err(|error| Box::new(Error::Send(JsError(error))))?)
                }
                _ => unreachable!("Bad ready state"),
            }
        } else if let Some(error) = self.error.borrow().as_ref() {
            Err(Box::new(error.clone()))
        } else {
            unreachable!("Bad transport state")
        }
    }

    fn close(self) {
        if let Some(State { websocket, .. }) = self.state.borrow_mut().as_ref() {
            let _ = websocket.close();
        }
    }
}

impl<C> Drop for WebSocketTransport<C>
where
    C: Codec<ServerMessage, ClientMessage>,
{
    fn drop(&mut self) {
        if let Some(State { websocket, .. }) = self.state.borrow_mut().as_ref() {
            let _ = websocket.close();
        }
    }
}

impl<C> WebSocketTransport<C>
where
    C: Codec<ServerMessage, ClientMessage>,
{
    fn new(websocket: WebSocket, codec: C) -> Self {
        let error = Rc::new(RefCell::new(None));
        let close_error = error.clone();
        let recv_error = error.clone();
        let recv_codec = codec.clone();
        let (recv_tx, recv_rx) = mpsc::channel();
        let state = Rc::new(RefCell::new(Some(State::new(
            websocket,
            codec,
            recv_rx,
            move |err| {
                close_error.replace(Some(Error::ConnectFailed(err)));
            },
            move |message| {
                if let Ok(buffer) = message.clone().dyn_into::<ArrayBuffer>() {
                    let array = Uint8Array::new(&buffer);
                    let array = array.to_vec();
                    match recv_codec.deserialize(array.as_slice()) {
                        Ok(message) => {
                            if let Err(error) = recv_tx.send(message) {
                                warn!("Receive message without channel: {:?}", error.0);
                            }
                        }
                        Err(error) => {
                            recv_error.replace(Some(Error::Receive(error)));
                        }
                    }
                } else if let Ok(string) = message.clone().dyn_into::<JsString>() {
                    let string = String::from(string);
                    let array = string.as_bytes();
                    match recv_codec.deserialize(array) {
                        Ok(message) => {
                            if let Err(error) = recv_tx.send(message) {
                                warn!("Receive message without channel: {:?}", error.0);
                            }
                        }
                        Err(error) => {
                            recv_error.replace(Some(Error::Receive(error)));
                        }
                    }
                } else {
                    warn!("Received bad message: {:?}", message)
                }
            },
        ))));
        Self {
            state,
            error: error.clone(),
        }
    }

    fn failed(error: JsError) -> Self {
        Self {
            state: Rc::new(RefCell::new(None)),
            error: Rc::new(RefCell::new(Some(Error::ConnectFailed(error)))),
        }
    }
}

pub struct WebSocketTransportParam<C>
where
    C: Codec<ServerMessage, ClientMessage>,
{
    url: Url,
    codec: C,
}

impl<C> TransportParam for WebSocketTransportParam<C>
where
    C: Codec<ServerMessage, ClientMessage>,
{
    fn connect(&self) -> Box<dyn Transport> {
        let url = self.url.to_string();
        let url = String::from(url);
        let websocket = WebSocket::new(url.as_str());
        let transport = match websocket {
            Ok(websocket) => WebSocketTransport::new(websocket, self.codec.clone()),
            Err(error) => WebSocketTransport::failed(JsError::from(error)),
        };
        Box::new(transport)
    }
}

impl<C> WebSocketTransportParam<C>
where
    C: Codec<ServerMessage, ClientMessage>,
{
    pub fn new(url: Url, codec: C) -> Self {
        Self { url, codec }
    }
}
