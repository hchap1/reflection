use std::{io::{Read, Write}, net::{IpAddr, Ipv4Addr, TcpListener, TcpStream}, thread::{JoinHandle, spawn}};

use async_channel::{Receiver, Sender, unbounded};

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

        let listener = TcpListener::bind((IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 7878))?;

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

        Ok(())
    }

    fn recv(mut client: TcpStream, output: Sender<NetworkMessage>) -> Res<()> {
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

    fn send(mut client: TcpStream, input: Receiver<NetworkMessage>) -> Res<()> {
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
}
