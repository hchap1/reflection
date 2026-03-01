use std::{net::{Ipv4Addr}};
use tokio::spawn;
use tokio::task::JoinHandle;
use tokio::net::TcpStream;

use crate::{communication::{NetworkMessage, server::{PORT, IDENTIFIER, Server}}, error::{ChannelError, Res}, frontend::application::ApplicationError, util::channel::send};
use async_channel::Sender;
use async_channel::Receiver;
use async_channel::unbounded;

#[derive(Debug)]
pub struct Client {
    thread: JoinHandle<Res<()>>,
    sender: Sender<NetworkMessage>
}

impl Client {

    pub async fn spawn() -> Res<(Self, Receiver<NetworkMessage>)> {
        let (send_to_foreign_sender, send_to_foreign_receiver) = unbounded();
        let (recv_from_foreign_sender, recv_from_foreign_receiver) = unbounded();
        let send_to_foreign_sender_clone = send_to_foreign_sender.clone();
        let target_address = Self::discover().await?;
        let recv_stream = TcpStream::connect((target_address, PORT)).await?;

        Ok((
            Self {
                thread: spawn(Self::run(recv_stream, recv_from_foreign_sender, send_to_foreign_receiver, send_to_foreign_sender_clone)),
                sender: send_to_foreign_sender
            },
            recv_from_foreign_receiver
        ))
    }

    async fn discover() -> Res<Ipv4Addr> {
        udp_discovery::client::discover(IDENTIFIER, PORT).await;
        Err(ApplicationError::NoEndpoint.into())
    }

    async fn run(mut recv_stream: TcpStream, output: Sender<NetworkMessage>, input: Receiver<NetworkMessage>, input_sender: Sender<NetworkMessage>) -> Res<()> {
        let (read_half, write_half) = recv_stream.into_split();

        let recv_thread = spawn(Server::recv(read_half, output));
        let send_thread = spawn(Server::send(write_half, input));

        let _ = recv_thread.await;

        // Interupt send thread
        input_sender.send(NetworkMessage::TerminateThread).await.map_err(ChannelError::from)?;
        let _ = send_thread.await;

        Ok(())
    }

    pub fn yield_sender(&self) -> Sender<NetworkMessage> {
        self.sender.clone()
    }

    pub async fn send_with(sender: Sender<NetworkMessage>, network_message: NetworkMessage) -> Res<()> {
        send(network_message, &sender).await?;
        Ok(())
    }
}
