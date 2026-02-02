use async_channel::Receiver;
use iced::{Element, Task};
use rusqlite_async::database::Database;

use crate::{directories::create::Directories, error::Error, frontend::{message::Message, pages::select_album::SelectAlbumPage}};

pub struct Application {
    database: Database,
    database_error_output: Receiver<rusqlite_async::error::Error>,
    
    select_album_page: SelectAlbumPage
}

impl Application {

    pub fn new() -> Self {

        let directories = Directories::create_or_load().expect("[CRITICAL ERROR] Unable to find suitable directories location.");
        let (database, error_handle) = Database::new(directories.root);

        Self {
            database,
            database_error_output: error_handle,
            select_album_page: SelectAlbumPage::new()
        }
    }

    pub fn view(&self) -> Element<Message> {
        
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        
    }
}
