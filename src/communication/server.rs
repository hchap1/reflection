use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::tcp::{OwnedReadHalf, OwnedWriteHalf}, task::{JoinHandle, spawn}};
use tokio::net::TcpListener;
use std::net::{IpAddr, Ipv4Addr};
use async_channel::{Receiver, Sender, unbounded};
use std::sync::{Arc, Mutex};

pub const IDENTIFIER: &str = "reflection";
pub const PORT: u16 = 7878;

use crate::{communication::NetworkMessage, error::{ChannelError, Res}, frontend::application::ApplicationError, util::channel::send};

pub struct Server {
    thread: JoinHandle<Res<()>>,
    sender: Sender<NetworkMessage>,
    active_connection: Arc<Mutex<Option<IpAddr>>>
}

impl Server {


    pub fn spawn() -> (Self, Receiver<NetworkMessage>) {

        let (send_to_foreign_sender, send_to_foreign_receiver) = unbounded();
        let (recv_from_foreign_sender, recv_from_foreign_receiver) = unbounded();
        let send_to_foreign_sender_clone = send_to_foreign_sender.clone();

        let active_connection = Arc::new(Mutex::new(None));
        let active_connection_clone = active_connection.clone();

        (
            Self {
                thread: spawn(Self::run(recv_from_foreign_sender, send_to_foreign_receiver, send_to_foreign_sender_clone, active_connection_clone)),
                sender: send_to_foreign_sender,
                active_connection
            },
            recv_from_foreign_receiver
        )
    }

    async fn run(output: Sender<NetworkMessage>, input: Receiver<NetworkMessage>, input_sender: Sender<NetworkMessage>, active_connection: Arc<Mutex<Option<IpAddr>>>) -> Res<()> {

        let listener = TcpListener::bind((IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), PORT)).await?;
        let udp = udp_discovery::server::Server::spawn(IDENTIFIER, PORT).await;

        while let Ok((tcp_stream, addr)) = listener.accept().await {

            {
                let mut active_connection = active_connection.lock().unwrap();
                active_connection.replace(addr.ip());
            }

            // Clones
            let (read_half, write_half) = tcp_stream.into_split();
            let output_clone = output.clone();
            let input_clone = input.clone();

            let recv_thread = spawn(Self::recv(read_half, output_clone));
            let send_thread = spawn(Self::send(write_half, input_clone));

            let _ = recv_thread.await;

            // Interupt send thread
            input_sender.send_blocking(NetworkMessage::TerminateThread).map_err(ChannelError::from)?;
            let _ = send_thread.await;

            // Clear channel
            while input.try_recv().is_ok() {}

            // Reset active connection
            {
                let mut active_connection = active_connection.lock().unwrap();
                _ = active_connection.take();
            }
        }

        udp.stop();
        let _ = udp.wait().await;
        Ok(())
    }

    pub async fn recv(mut client: OwnedReadHalf, output: Sender<NetworkMessage>) -> Res<()> {
        let mut size_buf = vec![0u8; 4];

        loop {
            client.read_exact(&mut size_buf).await?;
            let packet_size = u32::from_be_bytes(size_buf.clone().try_into().map_err(|_| ApplicationError::EndianFailure)?);
            let mut buf = vec![0u8; packet_size as usize];
            client.read_exact(&mut buf).await?;

            let network_message = NetworkMessage::from_bytes(&buf)?;
            output.send(network_message).await.map_err(ChannelError::from)?;
        }
    }

    pub async fn send(mut client: OwnedWriteHalf, input: Receiver<NetworkMessage>) -> Res<()> {
        while let Ok(message) = input.recv_blocking() {

            if let NetworkMessage::TerminateThread = message {
                client.shutdown().await?;
                break;
            }

            let bytes = message.to_bytes()?;
            let size: u32 = bytes.len() as u32;
            let endians = size.to_be_bytes().to_vec();

            client.write_all(&endians).await?;
            client.write_all(&bytes).await?;
        }


        Ok(())
    }

    pub fn get_active_connection(&self) -> Option<IpAddr> {
        let connection = self.active_connection.lock().unwrap();
        connection.clone()
    }

    pub fn get_sender(&self) -> Sender<NetworkMessage> {
        self.sender.clone()
    }

    pub async fn send_network_message(sender: Sender<NetworkMessage>, message: NetworkMessage) -> Res<()> {
        send(message, &sender).await?;
        Ok(())
    }
}
