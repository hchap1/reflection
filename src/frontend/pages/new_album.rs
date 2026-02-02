use iced::Task;
use iced::widget::Column;
use iced::widget::Scrollable;
use iced::widget::text;

use crate::frontend::message::Message;
use crate::frontend::widgets::photo_widget::PhotoWidget;
use crate::onedrive::get_album_children::Album;
use crate::onedrive::get_album_children::Photo;

#[derive(Debug, Clone)]
pub enum NewAlbumMessage {
    Display(Album, Vec<Photo>)
}

#[derive(Default)]
pub struct NewAlbumPage {
    album: Option<Album>,
    photos: Vec<Photo>
}

impl NewAlbumPage {
    pub fn view(&self) -> Column<Message> {
        Column::new()
            .spacing(10)
            .padding(10)
            .push(
                self.album.as_ref().map(|album| text(&album.name))
            ).push(
                self.album.as_ref().map(|album| text(&album.onedrive_id))
            ).push(
                self.album.as_ref().map(|album| text(&album.share_link))
            ).push(
                Scrollable::new(
                    Column::from_iter(self.photos.iter().map(|photo| PhotoWidget::list(photo, &None).into()))
                )
            )
    }

    pub fn update(&mut self, message: NewAlbumMessage) -> Task<Message> {

    }
}
