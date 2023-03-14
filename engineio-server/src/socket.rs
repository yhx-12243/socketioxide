use std::ops::ControlFlow;

use futures::{stream::SplitSink, SinkExt};
use hyper::upgrade::Upgraded;
use tokio::time::Timeout;
use tokio_tungstenite::{tungstenite, WebSocketStream};

use crate::{errors::Error, packet::Packet};

#[derive(Debug)]
pub struct Socket {
    sid: i64,
    http_tx: Option<hyper::body::Sender>,
    ws_tx: Option<SplitSink<WebSocketStream<Upgraded>, tungstenite::Message>>,
    ping_timeout: Option<Timeout<()>>,
}

impl Socket {
    pub fn new_http(sid: i64, sender: hyper::body::Sender) -> Self {
        Self {
            sid,
            http_tx: Some(sender),
            ws_tx: None,
            ping_timeout: None,
        }
    }
    pub fn new_ws(
        sid: i64,
        sender: SplitSink<WebSocketStream<Upgraded>, tungstenite::Message>,
    ) -> Self {
        let socket = Self {
            sid,
            http_tx: None,
            ws_tx: Some(sender),
            ping_timeout: None,
        };
        socket
    }

    pub fn upgrade_from_http(
        &mut self,
        tx: SplitSink<WebSocketStream<Upgraded>, tungstenite::Message>,
    ) {
        self.http_tx = None;
        self.ws_tx = Some(tx);
    }

    pub fn is_http(&self) -> bool {
        self.http_tx.is_some()
    }
    pub fn is_ws(&self) -> bool {
        self.ws_tx.is_some()
    }

    pub async fn handle_packet(
        &mut self,
        packet: Packet,
    ) -> ControlFlow<Result<(), Error>, Result<(), Error>> {
        println!("Received packet from conn http({}): {:?}", self.is_http(), packet);
        match packet {
            Packet::Open(_) => ControlFlow::Continue(Err(Error::BadPacket(
                "Unexpected Open packet, it should be only used in upgrade process",
            ))),
            Packet::Close => ControlFlow::Break(Ok(())),
            Packet::Ping => ControlFlow::Continue(Err(Error::BadPacket("Unexpected Ping packet"))),
            Packet::Pong => todo!(),
            Packet::Message(msg) => {
                println!("Received message: {}", msg);
                ControlFlow::Continue(Ok(()))
            }
            Packet::Upgrade => ControlFlow::Continue(Err(Error::BadPacket(
                "Unexpected Upgrade packet, upgrade from ws connection not supported",
            ))),
            Packet::Noop => ControlFlow::Continue(Err(Error::BadPacket(
                "Unexpected Noop packet, it should be only used in upgrade process",
            ))),
        }
    }
    pub async fn close(mut self) -> Result<(), Error> {
        if let Some(tx) = self.http_tx {
            self.http_tx = None;
            tx.abort();
        }
        if let Some(mut tx) = self.ws_tx {
            self.ws_tx = None;
            return tx.close().await.map_err(|e| Error::from(e));
        }
        Ok(())
    }

    pub async fn send(&mut self, packet: Packet) -> Result<(), Error> {
        let msg = packet.try_into().map_err(Error::from)?;
        if let Some(tx) = &mut self.http_tx {
            tx.send_data(hyper::body::Bytes::from(msg))
                .await
                .map_err(Error::from)?;
        } else if let Some(tx) = &mut self.ws_tx {
            tx.send(tungstenite::Message::Text(msg))
                .await
                .map_err(Error::from)?;
        }
        Ok(())
    }
}
