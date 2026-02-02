pub mod select_album;
pub mod photo_display;
pub mod browse_album;

#[derive(Clone, Debug)]
pub enum Pages {
    SelectAlbum,
    PhotoDisplay,
    BrowseAlbum
}
