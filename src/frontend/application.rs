use async_channel::Receiver;
use iced::{widget::text, Element, Task};
use rusqlite_async::database::Database;

use crate::authentication::oauth2::api::TokenSet;
use crate::authentication::oauth2::wrapper::authenticate;
use crate::database::interface;
use crate::directories::create::Directories;
use crate::error::Error;
use crate::frontend::message::Global;
use crate::frontend::message::Message;
use crate::frontend::pages::browse_album::BrowseAlbumPage;
use crate::frontend::pages::browse_album::BrowseAlbumMessage;
use crate::frontend::pages::select_album::SelectAlbumPage;
use crate::frontend::pages::select_album::SelectAlbumMessage;
use crate::frontend::pages::Pages;
use crate::onedrive::api::AccessToken;
use crate::onedrive::download::download_drive_item;
use crate::onedrive::get_album_children::new_album;
use crate::onedrive::get_drive::DriveData;

#[derive(Debug, Clone)]
pub enum ApplicationError {
    NotAuthenticated,
    NoSuchAlbum
}

pub struct Application {
    database: Database,
    database_error_output: Receiver<rusqlite_async::error::Error>,
    directories: Directories,

    // Authentication
    tokenset: Option<TokenSet>,
    drivedata: Option<DriveData>,

    // Pages
    active_page: Pages,
    select_album_page: SelectAlbumPage,
    browse_album_page: BrowseAlbumPage
}

impl Application {

    pub fn new() -> Self {

        let directories = Directories::create_or_load().expect("[CRITICAL ERROR] Unable to find suitable directories location.");
        let (database, error_handle) = Database::new(directories.root.clone());

        interface::create_tables(database.derive()).expect("[CRITICAL ERROR] Unable to create database tables.");

        Self {
            database,
            database_error_output: error_handle,
            directories,
            tokenset: None,
            drivedata: None,
            active_page: Pages::SelectAlbum,
            select_album_page: SelectAlbumPage::new(),
            browse_album_page: BrowseAlbumPage::default()
        }
    }

    #[allow(mismatched_lifetime_syntaxes)]
    pub fn view(&self) -> Element<Message> {
        match self.active_page {
            Pages::SelectAlbum => self.select_album_page.view().into(),
            Pages::PhotoDisplay => text("404").into(),
            Pages::BrowseAlbum => self.browse_album_page.view().into(),
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {

        match message {
            Message::Global(message) => {
                match message {
                    Global::None => Task::none(),

                    Global::Authenticate => {
                        let datalink = self.database.derive();
                        Task::batch(vec![
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
                                ),
                            Task::future(interface::select_albums(self.database.derive()))
                                .then(|res|
                                    match res {
                                        Ok(albums) => Task::batch(
                                            albums.into_iter()
                                                .map(|album| Task::done(SelectAlbumMessage::AddAlbum(album).into()))
                                        ),

                                        Err(error) => Task::done(error.into())
                                    }
                                )
                        ])
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
                                                    Task::done(BrowseAlbumMessage::Display(album, contents).into()),
                                                    Task::done(Global::Load(Pages::BrowseAlbum).into())
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
                        self.tokenset = Some(tokenset);
                        self.drivedata = Some(drivedata);
                        Task::none()
                    }

                    Global::Download(photo, album_id) => {
                        let access_token = match self.tokenset.as_ref() {
                            Some(tokenset) => AccessToken::new(tokenset.access_token.clone()),
                            None => return Task::done(Error::from(ApplicationError::NotAuthenticated).into())
                        };

                        let photo_id_clone = photo.onedrive_id.clone();

                        Task::future(download_drive_item(access_token, photo, self.directories.albums.clone(), album_id))
                            .then(move |res| match res {
                                Ok((_image_path, thumbnail_option)) => Task::batch(vec![
                                    Task::done(match thumbnail_option {
                                        Some(thumbnail_path) => BrowseAlbumMessage::Thumbnail(photo_id_clone.clone(), thumbnail_path).into(),
                                        None => Global::None.into()
                                    })
                                ]),
                                Err(error) => Task::done(error.into())
                            })
                    }

                    Global::Load(page) => {
                        self.active_page = page;
                        Task::none()
                    }

                    Global::BrowseAlbum(album_sql_id) => {
                        let datalink = self.database.derive();
                        Task::done(Global::Load(Pages::BrowseAlbum).into()).chain(Task::future(interface::select_photos_in_album(datalink, album_sql_id))
                            .then(|res| match res {
                                Ok((album, contents)) => Task::done(BrowseAlbumMessage::Display(album.clone(), contents.clone()).into()).chain({
                                    let datalink = self.database.derive();
                                    Task::batch(
                                        contents.into_iter()
                                            .map(|photo| Task::done())
                                    )
                                }),
                                Err(error) => Task::done(error.into())
                            })
                        )
                    }
                }
            },

            Message::SelectAlbumMessage(message) => self.select_album_page.update(message),
            Message::BrowseAlbumMessage(message) => self.browse_album_page.update(message),

            Message::Error(error) => {
                eprintln!("Error: {error:?}");
                Task::none()
            }
        }
    }
}
