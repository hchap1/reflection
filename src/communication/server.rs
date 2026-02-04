use std::{io::{Read, Write}, net::{IpAddr, Ipv4Addr, TcpListener, TcpStream}, thread::{JoinHandle, spawn}};
use async_channel::{Receiver, Sender, unbounded};
use if_addrs::get_if_addrs;
use mdns_sd::{ServiceDaemon, ServiceInfo};

pub const PORT: u16 = 7878;
pub const SERVICE_TYPE: &str = "_reflection._tcp.local.";
pub const INSTANCE_NAME: &str = "reflection";
pub const PROPERTIES: [(&str, &str); 1] = [("version", "1.0")];

use crate::{communication::NetworkMessage, error::{ChannelError, Res}, frontend::application::ApplicationError};

pub struct Server {
    thread: JoinHandle<Res<()>>,
    sender: Sender<NetworkMessage>
}

impl Server {


    pub fn spawn() -> (Self, Receiver<NetworkMessage>) {

        let (send_to_foreign_sender, send_to_foreign_receiver) = unbounded();
        let (recv_from_foreign_sender, recv_from_foreign_receiver) = unbounded();
        let send_to_foreign_sender_clone = send_to_foreign_sender.clone();

        (
            Self {
                thread: spawn(move || Self::run(recv_from_foreign_sender, send_to_foreign_receiver, send_to_foreign_sender_clone)),
                sender: send_to_foreign_sender
            },
            recv_from_foreign_receiver
        )
    }

    fn run(output: Sender<NetworkMessage>, input: Receiver<NetworkMessage>, input_sender: Sender<NetworkMessage>) -> Res<()> {

        let listener = TcpListener::bind((IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), PORT))?;
        let (mdns_sender, mdns_receiver) = unbounded();

        let mdns_thread = spawn(move || Self::advertise_service(mdns_receiver));

        while let Ok((tcp_stream, _)) = listener.accept() {

            // Clones
            let tcp_stream_clone = tcp_stream.try_clone()?;
            let output_clone = output.clone();
            let input_clone = input.clone();

            let recv_thread = spawn(move || Self::recv(tcp_stream_clone, output_clone));
            let send_thread = spawn(move || Self::send(tcp_stream, input_clone));

            let _ = recv_thread.join();

            // Interupt send thread
            input_sender.send_blocking(NetworkMessage::TerminateThread).map_err(ChannelError::from)?;
            let _ = send_thread.join();

            // Clear channel
            while input.try_recv().is_ok() {}
        }

        mdns_sender.send_blocking(()).map_err(ChannelError::from)?;
        let _ = mdns_thread.join();

        Ok(())
    }

    pub fn recv(mut client: TcpStream, output: Sender<NetworkMessage>) -> Res<()> {
        let mut size_buf = vec![0u8; 4];

        loop {
            client.read_exact(&mut size_buf)?;
            let packet_size = u32::from_be_bytes(size_buf.clone().try_into().map_err(|_| ApplicationError::EndianFailure)?);
            let mut buf = vec![0u8; packet_size as usize];
            client.read_exact(&mut buf)?;

            let network_message = NetworkMessage::from_bytes(&buf)?;
            output.send_blocking(network_message).map_err(ChannelError::from)?;
        }
    }

    pub fn send(mut client: TcpStream, input: Receiver<NetworkMessage>) -> Res<()> {
        while let Ok(message) = input.recv_blocking() {

            if let NetworkMessage::TerminateThread = message {
                client.shutdown(std::net::Shutdown::Both)?;
                break;
            }

            let bytes = message.to_bytes()?;
            let size: u32 = bytes.len() as u32;
            let endians = size.to_be_bytes().to_vec();

            client.write_all(&endians)?;
            client.write_all(&bytes)?;
        }


        Ok(())
    }

    fn get_lan_ip() -> Res<IpAddr> {
        for iface in get_if_addrs()? {
            if !iface.is_loopback() && iface.ip().is_ipv4() {
                return Ok(iface.ip())
            }
        }

        Err(ApplicationError::NoEndpoint)?
    }

    fn advertise_service(shutdown_receiver: Receiver<()>) -> Res<()> {

        let ip = Self::get_lan_ip()?;

        let ip_string = ip.to_string();
        let hostname = format!("{ip_string}.local");

        let mdns = ServiceDaemon::new()?;

        let service_info = ServiceInfo::new(
            SERVICE_TYPE,
            INSTANCE_NAME,
            &hostname,
            ip,
            PORT,
            &PROPERTIES[..]
        )?;

        mdns.register(service_info)?;
        let _ = shutdown_receiver.recv_blocking();
        mdns.shutdown()?;

        Ok(())
    }
}
