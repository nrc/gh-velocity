use crate::config::{ACCESS_TOKEN, OWNER, REPO, USER_AGENT};
use crate::data::{self, Date, Sha, Status};
use crate::{db, Result};

use futures::compat::Compat01As03;
use futures::prelude::*;
use hubcaps::{
    self,
    issues::State,
    pulls::{Pull, PullListOptions},
    Credentials, Github,
};
use std::convert::TryFrom;

pub fn update_from_repo() {
    // TODO handle any errors
    futures::executor::block_on(open_pull_requests().then(record_data).collect::<Vec<_>>());
    // TODO for any PR which we think is still open, but isn't in the above result, query GitHub to get a sample for when it was closed/merged
}

fn open_pull_requests() -> impl Stream<Item = hubcaps::Result<Pull>> {
    let github = Github::new(
        USER_AGENT.to_owned(),
        Credentials::Token(ACCESS_TOKEN.to_owned()),
    );
    let opts = PullListOptions::builder().state(State::Open).build();
    Compat01As03::new(
        github
            .repo(OWNER.to_owned(), REPO.to_owned())
            .pulls()
            .iter(&opts),
    )
}

async fn record_data(p: hubcaps::Result<Pull>) -> Result<()> {
    let p = p?;

    let github = Github::new(
        USER_AGENT.to_owned(),
        Credentials::Token(ACCESS_TOKEN.to_owned()),
    );
    let pull = github
        .repo(OWNER.to_owned(), REPO.to_owned())
        .pulls()
        .get(p.number);
    let mut commits = Compat01As03::new(pull.commits().iter());
    let first_commit = commits.next().await;
    let first_sha = first_commit
        .map::<Result<_>, _>(|c| Ok(c?.sha))
        .unwrap_or_else(|| Ok(String::new()))?;
    let review_comments = Compat01As03::new(pull.review_comments().list())
        .await?
        .len();
    record_sample::<db::DeployConnProvider>(p, first_sha, review_comments)
}

fn record_sample<T: db::ConnectionProvider>(
    pull: Pull,
    first_sha: String,
    review_comments: usize,
) -> Result<()> {
    let conn = T::connection()?;

    let author = data::User {
        id: saturating_from(pull.user.id),
        username: pull.user.login,
        url: pull.user.url,
    };
    author.insert_into(&conn)?;

    let pr = data::PullRequest {
        id: u32::try_from(pull.id).unwrap_or_else(|_| u32::max_value()),
        number: u32::try_from(pull.number).unwrap_or_else(|_| u32::max_value()),
        title: pull.title,
        body: pull.body.unwrap_or_else(String::new),
        author,
        created: Date::new(pull.created_at),
        url: pull.url,
    };
    pr.insert_into(&conn)?;

    let sample = data::Sample {
        time: Date::new(pull.updated_at),
        pr,
        status: Status::from_opts(pull.closed_at, pull.merged_at),
        commits: saturating_from_opt(pull.commits),
        additions: saturating_from_opt(pull.additions),
        deletions: saturating_from_opt(pull.deletions),
        changed_files: saturating_from_opt(pull.changed_files),
        review_comments: saturating_from(review_comments),
        first_commit: Sha(first_sha),
    };
    sample.insert_into(&conn)?;

    Ok(())
}

#[inline]
fn saturating_from<T>(v: T) -> u32
where
    u32: TryFrom<T>,
{
    u32::try_from(v).unwrap_or_else(|_| u32::max_value())
}

#[inline]
fn saturating_from_opt<T>(v: Option<T>) -> u32
where
    u32: TryFrom<T>,
{
    v.map(|v| u32::try_from(v).unwrap_or_else(|_| u32::max_value()))
        .unwrap_or(0)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sat_from() {
        assert_eq!(42, saturating_from::<u64>(42));
        assert_eq!(u32::max_value(), saturating_from::<u64>(u32::max_value() as u64 + 500));
        assert_eq!(42, saturating_from_opt::<u64>(Some(42)));
        assert_eq!(u32::max_value(), saturating_from_opt::<u64>(Some(u32::max_value() as u64 + 500)));
        assert_eq!(0, saturating_from_opt::<u64>(None));
    }

    #[test]
    fn test_record_sample() {
        // TODO 
        unimplemented!();
    }
}
