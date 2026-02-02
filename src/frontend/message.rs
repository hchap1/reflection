use crate::frontend::pages::select_album::SelectAlbumMessage;

macro_rules! message_enum {
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
                $variant($variant),
            )*
        }

        $(
            impl From<$variant> for $name {
                fn from(e: $variant) -> Self {
                    $name::$variant(e)
                }
            }
        )*
    };
}

#[derive(Clone, Debug)]
pub enum Global {
    None,
    // Called to start async tasks to authenticate, using either database or oauth2
    Authenticate,

    AddNewAlbum(String),
}

message_enum! {
    pub enum Message {
        Global,
        SelectAlbumMessage
    }
}
