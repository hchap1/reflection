use std::sync::Arc;
use async_channel::Receiver;

use crate::communication::NetworkMessage;
use crate::communication::client::Client;
use crate::error::Error;

#[derive(Clone, Debug)]
pub enum Message {
    None,

    // Attempt to form a connection with the display server
    Connect,
    Connected(Arc<Client>, Receiver<NetworkMessage>),

    // Perform an OAUTH2 authentication, and relay to display server
    Authenticate,
    
    // Process messages to and from the display server
    IncomingNetworkMessage(NetworkMessage),
    OutgoingNetworkMessage(NetworkMessage),

    // GUI
    Hover(usize),
    Unhover(usize),

    Error(Error)
}
