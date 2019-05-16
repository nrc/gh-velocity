//! Data received from GitHub to be inserted into the database.

pub struct Sample {
    pub time: Date,
    pub pr: PullRequest,
    pub status: Status,
    pub commits: u32,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: u32,
    pub review_comments: u32,
    pub first_commit: Sha,
}

pub struct PullRequest {
    pub id: u32,
    pub number: u32,
    pub title: String,
    pub body: String,
    pub author: User,
    pub created: Date,
    pub url: String,
}

pub struct User {
    pub id: u32,
    pub username: String,
    pub url: String,
}

// TODO perhaps move out the below stuff (or separate generic data from GH data)
#[derive(Debug, Eq, PartialEq)]
pub enum Status {
    Open,
    Closed(Date),
    Merged(Date),
}

impl Status {
    pub fn from_opts(closed: Option<String>, merged: Option<String>) -> Status {
        match (closed, merged) {
            (None, None) => Status::Open,
            (_, Some(s)) => Status::Merged(Date::new(s)),
            (Some(s), _) => Status::Closed(Date::new(s)),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Date {
    pub date: String,
}

impl Date {
    pub fn new(date: String) -> Date {
        Date { date }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Sha(pub String);
