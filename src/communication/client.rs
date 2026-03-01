use std::{net::{IpAddr, Ipv4Addr, TcpStream}, thread::spawn};
use std::thread::JoinHandle;

use crate::{communication::{NetworkMessage, server::{PORT, SERVICE_TYPE, Server}}, error::{ChannelError, Res}, frontend::application::ApplicationError, util::channel::send};
use async_channel::Sender;
use async_channel::Receiver;
use async_channel::unbounded;
use mdns_sd::ServiceDaemon;

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

        let target_address = tokio::task::spawn_blocking(Self::discover).await??;
        let recv_stream = tokio::task::spawn_blocking(move || TcpStream::connect((target_address, PORT))).await??;

        Ok((
            Self {
                thread: spawn(move || Self::run(recv_stream, recv_from_foreign_sender, send_to_foreign_receiver, send_to_foreign_sender_clone)),
                sender: send_to_foreign_sender
            },
            recv_from_foreign_receiver
        ))
    }

    fn discover() -> Res<Ipv4Addr> {
        let mdns = ServiceDaemon::new()?;
        let receiver = mdns.browse(SERVICE_TYPE)?;

        while let Ok(event) = receiver.recv() {
            match event {
                mdns_sd::ServiceEvent::ServiceResolved(service) => return service
                    .addresses
                    .into_iter()
                    .filter_map(
                        |scoped_ip|
                        if let IpAddr::V4(ipv4) = scoped_ip.to_ip_addr() {
                            Some(ipv4)
                        } else { None }
                    )
                    .next()
                    .ok_or(ApplicationError::NoEndpoint.into()),
                _ => continue
            }
        };

        Err(ApplicationError::NoEndpoint.into())
    }

    fn run(recv_stream: TcpStream, output: Sender<NetworkMessage>, input: Receiver<NetworkMessage>, input_sender: Sender<NetworkMessage>) -> Res<()> {
        let send_stream = recv_stream.try_clone()?;

        let recv_thread = spawn(move || Server::recv(recv_stream, output));
        let send_thread = spawn(move || Server::send(send_stream, input));

        let _ = recv_thread.join();

        // Interupt send thread
        input_sender.send_blocking(NetworkMessage::TerminateThread).map_err(ChannelError::from)?;
        let _ = send_thread.join();

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
