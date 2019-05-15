#![feature(async_await)]

use std::{thread, time::Duration};

use futures::compat::Compat01As03;
use hubcaps::{issues::State, pulls::PullListOptions, Credentials, Github};
use rusqlite;

mod data;
mod db;

/// Time between updates in seconds
const UPDATE_TIMEOUT: u64 = 60 * 60;

const USER_AGENT: &str = "gh-velocity";
const ACCESS_TOKEN: &str = "TODO personal-access-token";
const OWNER: &str = "nrc";
const REPO: &str = "gh-velocity";
const DB_PATH: &str = "ghv-staging.db";

fn ensure_pr(number: u32) {}

fn record_sample() {}

fn update_from_repo() {
    let github = Github::new(
        USER_AGENT.to_owned(),
        Credentials::Token(ACCESS_TOKEN.to_owned()),
    );
    let opts = PullListOptions::builder().state(State::Open).build();
    let pulls = Compat01As03::new(
        github
            .repo(OWNER.to_owned(), REPO.to_owned())
            .pulls()
            .iter(&opts),
    );
    // TODO do something with the pulls
}

/// Update from GitHub every `UPDATE_TIMEOUT`s.
fn update_loop() {
    loop {
        update_from_repo();
        thread::sleep(Duration::from_secs(UPDATE_TIMEOUT));
    }
}

#[derive(Debug)]
pub enum GhvError {
    DbError(rusqlite::Error),
}

impl From<rusqlite::Error> for GhvError {
    fn from(e: rusqlite::Error) -> GhvError {
        GhvError::DbError(e)
    }
}

type Result<T> = ::std::result::Result<T, GhvError>;

fn main() {
    let conn = db::connection().unwrap();
    db::init(&conn).unwrap();
    // update_loop();
}
