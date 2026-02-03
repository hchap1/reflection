use std::time::UNIX_EPOCH;

use chrono::{DateTime, Local};
use iced::widget::{Column, image};
use iced::widget::text;
use iced::widget::Row;
use iced::advanced::image::Handle;
use iced::widget::Container;

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
                                        let time = UNIX_EPOCH + std::time::Duration::new(date, 0);
                                        let datetime: DateTime<Local> = time.into();
                                        datetime.format("%Y-%m-%d %H:%M:%S").to_string()
                                    }))
                                )
                        )
                )
        )
    }
}
