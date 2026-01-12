pub type Res<T> = Result<T, Error>;

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

error_enum! {
    pub enum Error {

    }
}
