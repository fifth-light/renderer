use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

use server::{websocket::WebSocketServer, Server};
use tokio_serde::formats::Json;

pub mod entity;
pub mod server;
pub mod world;

#[tokio::main]
async fn main() {
    env_logger::init();
    let server = Arc::new(Server::default());

    let serve = {
        let server = server.clone();
        tokio::spawn(async move {
            let websocket_werver =
                WebSocketServer::new(SocketAddr::new(IpAddr::V4(Ipv4Addr::UNSPECIFIED), 12345));
            websocket_werver.serve(server, Json::default).await
        })
    };

    let run = {
        let server = server.clone();
        tokio::spawn(async move { server.run().await })
    };

    tokio::select! {
        serve_result = serve => {
            let serve_result = serve_result.expect("Serve crashed");
            if let Err(err) = serve_result {
                panic!("Failed to serve: {:?}", err)
            }
        }
        run_result = run => {
            run_result.expect("Run crashed");
        }
    };
}
