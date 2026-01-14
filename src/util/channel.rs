use async_channel::Sender;
use crate::error::ChannelError;

pub async fn send<T>(message: T, sender: &Sender<T>) -> Result<(), ChannelError> {
    Ok(sender.send(message).await?)
}
