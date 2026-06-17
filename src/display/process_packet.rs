use iced::Task;
use rkyv::Archived;
use rkyv::access;
use lan_tcp::networking::node::RecvPacket;

use crate::backend::database::sql::SQL;
use crate::backend::networking::network_message::ArchivedControlToDisplay;
use crate::backend::networking::network_message::ControlToDisplay;
use crate::backend::networking::network_message::DisplayToControl;
use crate::display::application::Message;
use crate::error::Res;

pub fn process_packet(recv_packet: RecvPacket) -> Res<Task<Message>> {

    // It is expected that recv_packet contains a ControlToDisplay
    let control_to_display: &Archived<ControlToDisplay> = access::<
        Archived<ControlToDisplay>,
        rkyv::rancor::Error
    >(&recv_packet.data)?;

    // Process the command
    let task = match control_to_display {

        ArchivedControlToDisplay::RequestUsers => Task::perform(
            SQL::select_all_users(),
            |res| match res {
                Ok(users) => Message::Batch(
                    users.into_iter()
                        .map(|user| Message::Send(
                            DisplayToControl::UserInformation(user.into())
                        ))
                        .collect()
                ),
                Err(e) => Message::Error(e)
            }
        ),

        _ => todo!("Implement.")
    };

    Ok(task)

}
