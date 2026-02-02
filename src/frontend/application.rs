use async_channel::Receiver;
use iced::{widget::text, Element, Task};
use rusqlite_async::database::Database;

use crate::{directories::create::Directories, error::Error, frontend::{message::Message, pages::select_album::SelectAlbumPage}};
use crate::{authentication::oauth2::{api::TokenSet, wrapper::authenticate}, directories::create::Directories, error::Error, frontend::{message::{Global, Message}, pages::{new_album, select_album::{SelectAlbumMessage, SelectAlbumPage}, Pages}}, onedrive::{api::AccessToken, get_album_children::{self, new_album}, get_drive::DriveData}};

pub enum ApplicationError {
    NotAuthenticated
}

pub struct Application {
    database: Database,
    database_error_output: Receiver<rusqlite_async::error::Error>,
    select_album_page: SelectAlbumPage

    // Authentication
    tokenset: Option<TokenSet>,
    drivedata: Option<DriveData>,

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
            tokenset: None,
            drivedata: None,
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
                        match (self.tokenset.as_ref(), self.drivedata.as_ref()) {
                            (Some(tokenset), Some(drivedata)) => {
                                let access_token = tokenset.access_token.clone();
                                let drive_id = drivedata.id.clone();
                                let datalink = self.database.derive();
                                Task::future(new_album(AccessToken::new(access_token), drive_id, sharelink, datalink))
                                    .then(|res|
                                        match res {
                                            Ok((album, contents)) => {
                                                Task::batch(vec![
                                                    Task::done(SelectAlbumMessage::AddAlbum(album.clone()).into()),
                                                    // TODO load new album confirm page with given information
                                                    Task::done()
                                                ])
                                            }

                                            Err(error) => Task::done(error.into())
                                        }
                                    )
                            }

                            _ => Task::done(Error::from(ApplicationError::NotAuthenticated).into())
                        }
                    }

                    Global::AuthenticationComplete(tokenset, drivedata) => {

                    }
                }
            },

            Message::SelectAlbumMessage(message) => self.select_album_page.update(message),
        }
    }
}
