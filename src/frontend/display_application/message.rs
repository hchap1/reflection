use crate::authentication::oauth2::api::TokenSet;
use crate::communication::NetworkMessage;
use crate::error::Error;
use crate::onedrive::get_drive::DriveData;

#[derive(Clone, Debug)]
pub enum Message {
    None,

    // Process messages to and from the control application
    IncomingNetworkMessage(NetworkMessage),
    OutgoingNetworkMessage(NetworkMessage),

    // Save incoming authentication information
    AuthenticationComplete(TokenSet, DriveData),

    Error(Error)
}
