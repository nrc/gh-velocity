#![feature(async_await)]

use hubcaps;
use rusqlite;
use std::{env, thread, time::Duration};

use crate::frontend::Blob;

mod config;
mod data;
mod db;
mod frontend;
mod github;

/// Update from GitHub every `UPDATE_TIMEOUT`s.
fn update_loop(blob: Blob) {
    loop {
        github::update_from_repo();
        // TODO deal with errors?
        blob.update();
        thread::sleep(Duration::from_secs(config::UPDATE_TIMEOUT));
    }
}

#[derive(Debug)]
pub enum GhvError {
    DbError(rusqlite::Error),
    GhError(hubcaps::Error),
    Other,
}

impl From<rusqlite::Error> for GhvError {
    fn from(e: rusqlite::Error) -> GhvError {
        GhvError::DbError(e)
    }
}

impl From<hubcaps::Error> for GhvError {
    fn from(e: hubcaps::Error) -> GhvError {
        GhvError::GhError(e)
    }
}

impl<T> From<std::sync::PoisonError<T>> for GhvError {
    fn from(_: std::sync::PoisonError<T>) -> GhvError {
        GhvError::Other
    }
}

type Result<T> = ::std::result::Result<T, GhvError>;

fn main() {
    let mut args = env::args();
    args.next().expect("No first argument?");
    if let Some(first_arg) = args.next() {
        if first_arg == "--init" {
            let conn = db::connection().expect("Could not connect to db");
            db::init(&conn).expect("Could not initialise db");
        }
    }

    let blob = Blob::new();
    update_loop(blob.clone());
    // TODO frontend thread
}
