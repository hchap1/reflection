use crate::communication::NetworkMessage;

#[derive(Clone, Debug)]
pub enum Message {

    // Attempt to form a connection with the display server
    Connect,

    // Perform an OAUTH2 authentication, and relay to display server
    Authenticate,
    
    // Process messages to and from the display server
    IncomingNetworkMessage(NetworkMessage),
    OutgoingNetworkMessage(NetworkMessage),

    // GUI
    Hover(usize),
    Unhover(usize),
}
