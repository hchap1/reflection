use crate::authentication::oauth2::api::TokenSet;
use crate::communication::NetworkMessage;
use crate::error::Error;

#[derive(Clone, Debug)]
pub enum Message {
    None,

    // Attempt to form a connection with the control application
    Connect,
    
    // Process messages to and from the control application
    IncomingNetworkMessage(NetworkMessage),
    OutgoingNetworkMessage(NetworkMessage),

    // Save incoming authentication information
    SaveAuthentication(TokenSet),

    Error(Error)
}
