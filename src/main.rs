#![allow(clippy::enum_variant_names)]

mod error;
mod util;
mod database;
mod directories;
mod authentication;
mod onedrive;
mod frontend;

fn main() {

}

// TODO Implement the rest of the SQL functions
// TODO Create async function to retrieve all photos from an album and insert them all into the database
// TODO Download scan function that checks every photo and makes sure they are downloaded
// TODO Create wrapper around album scan function that does it for every cached album
// TODO Begin work on frontend
