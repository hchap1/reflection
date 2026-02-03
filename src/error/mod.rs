use std::sync::Arc;
use std::time::SystemTimeError;
use async_channel::SendError;
use async_channel::RecvError;
use async_channel::TryRecvError;
use rand::rand_core::OsError;
use tokio::task::JoinError;

use crate::authentication::callback::server::ServerError;
use crate::database::interface::DatabaseInterfaceError;
use crate::directories::create::DirectoryError;
use crate::authentication::oauth2::api::OAUTH2ApiError;
use crate::frontend::application::ApplicationError;
use crate::onedrive::api::OnedriveError;
use crate::onedrive::download::DownloadError;

pub type Res<T> = Result<T, Error>;
type StdIoError = std::io::Error;
type HyperError = hyper::Error;
type ReqwestError = reqwest::Error;
type SerdeJsonError = serde_json::Error;
type DatabaseError = rusqlite_async::error::Error;
type RancorError = rkyv::rancor::Error;
type MdnsError = mdns_sd::Error;

macro_rules! error_enum {
    (
        $(#[$meta:meta])*
        $vis:vis enum $name:ident {
            $($variant:ident),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone)]
        $vis enum $name {
            $(
                $variant(Arc<$variant>),
            )*
        }

        $(
            impl From<$variant> for $name {
                fn from(e: $variant) -> Self {
                    $name::$variant(Arc::new(e))
                }
            }
        )*
    };
}

#[derive(Debug, Clone)]
pub enum ChannelError {
    ChannelDead,
    ChannelEmpty
}

impl<T> From<SendError<T>> for ChannelError {
    fn from(_: SendError<T>) -> ChannelError {
        ChannelError::ChannelDead
    }
}

impl From<RecvError> for ChannelError {
    fn from(_: RecvError) -> ChannelError {
        ChannelError::ChannelDead
    }
}

impl From<TryRecvError> for ChannelError {
    fn from(error: TryRecvError) -> ChannelError {
        match error {
            TryRecvError::Empty => ChannelError::ChannelEmpty,
            _ => ChannelError::ChannelDead
        }
    }
}

error_enum! {
    pub enum Error {
        StdIoError,
        ServerError,
        HyperError,
        ChannelError,
        OsError,
        JoinError,
        ReqwestError,
        SerdeJsonError,
        OAUTH2ApiError,
        DatabaseError,
        SystemTimeError,
        DatabaseInterfaceError,
        DirectoryError,
        OnedriveError,
        DownloadError,
        ApplicationError,
        RancorError,
        MdnsError
    }
}
