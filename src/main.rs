#![feature(async_await)]

use hubcaps;
use rusqlite;
use std::{env, thread, time::Duration};

mod data;
mod db;
mod github;

/// Time between updates in seconds
const UPDATE_TIMEOUT: u64 = 60 * 60;

const USER_AGENT: &str = "gh-velocity";
const ACCESS_TOKEN: &str = "TODO personal-access-token";
// TODO we should work across multiple orgs/repos
const OWNER: &str = "nrc";
const REPO: &str = "gh-velocity";
const DB_PATH: &str = "ghv-staging.db";

/// Update from GitHub every `UPDATE_TIMEOUT`s.
fn update_loop() {
    loop {
        github::update_from_repo();
        thread::sleep(Duration::from_secs(UPDATE_TIMEOUT));
    }
}

#[derive(Debug)]
pub enum GhvError {
    DbError(rusqlite::Error),
    GhError(hubcaps::Error),
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

    update_loop();
}
