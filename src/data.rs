//! Data received from GitHub to be inserted into the database.

pub struct Sample {
    pub time: Date,
    pub pr: PullRequest,
    pub stats: PrStats,
    pub status: Status,
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

pub struct PrStats {
    pub commits: u32,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: u32,
    pub review_comments: u32,
    pub reviewers: u32,
    pub first_commit: Sha,
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

#[derive(Debug, Eq, PartialEq)]
pub struct Date {
    pub date: String,
}

impl Date {
    pub fn new(date: String) -> Date {
        Date { date }
    }

    pub fn param_str(&self) -> &str {
        &self.date
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Sha(pub String);
