use chrono::{DateTime, Local};
use iced::widget::{Container, Row, image::Handle};
use iced::widget::{Column, image};
use iced::widget::text;

use crate::{frontend::message::Message, onedrive::get_album_children::Photo};

pub struct PhotoWidget;

impl PhotoWidget {
    pub fn list<'a>(photo: &'a Photo, thumbnail: Option<&'a Handle>) -> Container<'a, Message> {
        Container::new(
            Row::new()
                .spacing(10)
                .padding(10)
                .push(
                    thumbnail.as_ref().map(|thumbnail| image(*thumbnail))
                ).push(
                    Column::new()
                        .spacing(10)
                        .padding(10)
                        .push(
                            text(&photo.name)
                        ).push(
                            Row::new()
                                .spacing(10)
                                .padding(10)
                                .push(
                                    photo.creation_date.map(|date| text({
                                        let datetime: DateTime<Local> = date.into();
                                        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                                    }))
                                )
                        )
                )
        )
    }
}
