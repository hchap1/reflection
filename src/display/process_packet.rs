use rkyv::Archived;
use rkyv::access;
use lan_tcp::networking::node::RecvPacket;

use crate::backend::networking::network_message::ControlToDisplay;
use crate::display::application::Message;
use crate::error::Res;

pub async fn process_packet(recv_packet: RecvPacket) -> Res<Message> {

    // It is expected that recv_packet contains a ControlToDisplay
    // Use full deserialisation - performance here is not important

    let control_to_display: &Archived<ControlToDisplay> = access::<
        Archived<ControlToDisplay>,
        rkyv::rancor::Error
    >(&recv_packet.data)?;

    Ok(Message::None)

}
