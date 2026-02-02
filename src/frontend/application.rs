use async_channel::Receiver;
use iced::{widget::text, Element, Task};
use rusqlite_async::database::Database;

use crate::{authentication::oauth2::wrapper::authenticate, directories::create::Directories, error::Error, frontend::{message::{Global, Message}, pages::{select_album::SelectAlbumPage, Pages}}};

pub struct Application {
    database: Database,
    database_error_output: Receiver<rusqlite_async::error::Error>,

    // Pages
    active_page: Pages,
    select_album_page: SelectAlbumPage,
}

impl Application {

    pub fn new() -> Self {

        let directories = Directories::create_or_load().expect("[CRITICAL ERROR] Unable to find suitable directories location.");
        let (database, error_handle) = Database::new(directories.root);

        Self {
            database,
            database_error_output: error_handle,
            active_page: Pages::SelectAlbum,
            select_album_page: SelectAlbumPage::new()
        }
    }

    pub fn view(&self) -> Element<Message> {
        match self.active_page {
            Pages::SelectAlbum => self.select_album_page.view().into(),
            Pages::PhotoDisplay => text("404").into(),
            Pages::NewAlbum => text("404").into(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Global(message) => {
                match message {
                    Global::None => Task::none(),

                    Global::Authenticate => {
                        let datalink = self.database.derive();
                        Task::future(authenticate(datalink))
                            .map(|res|
                                match res {
                                    Ok((tokenset, drivedata)) => {
                                        Global::AuthenticationComplete(tokenset, drivedata).into()
                                    }

                                    Err(error) => {
                                        error.into()
                                    }
                                }
                            )
                    }

                    Global::AddNewAlbum(sharelink) => {

                    }
                }
            },

            Message::SelectAlbumMessage(message) => self.select_album_page.update(message),
        }
    }
}
