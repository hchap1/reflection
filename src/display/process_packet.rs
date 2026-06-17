use iced::Task;
use rkyv::Archived;
use rkyv::access;
use lan_tcp::networking::node::RecvPacket;

use crate::backend::database::database_backend::Database;
use crate::backend::networking::network_message::ArchivedControlToDisplay;
use crate::backend::networking::network_message::ControlToDisplay;
use crate::display::application::Message;
use crate::error::Res;

pub fn process_packet(recv_packet: RecvPacket) -> Res<Task<Message>> {

    // It is expected that recv_packet contains a ControlToDisplay
    let control_to_display: &Archived<ControlToDisplay> = access::<
        Archived<ControlToDisplay>,
        rkyv::rancor::Error
    >(&recv_packet.data)?;

    // Process the command
    match control_to_display {

        ArchivedControlToDisplay::RequestUsers =>

        _ => {}
    }

    Ok(Task::none())

}
