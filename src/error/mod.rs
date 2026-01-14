use std::sync::Arc;

use crate::callback::server::ServerError;

pub type Res<T> = Result<T, Error>;
type StdIoError = std::io::Error;
type HyperError = hyper::Error;

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

error_enum! {
    pub enum Error {
        StdIoError,
        ServerError,
        HyperError
    }
}
