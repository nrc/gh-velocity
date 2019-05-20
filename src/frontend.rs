use serde_derive::Serialize;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::{db, Result};

// TODO
// Use Tide to send it as json via an http endpoint (per block = n weeks, just do everything for now)
// Provide the web frontend (and static files) as another endpoint

pub(crate) type Blob = BlobOuter<db::DeployConnProvider>;

#[derive(Clone)]
pub(crate) struct BlobOuter<T: db::ConnectionProvider> {
    inner: Arc<Mutex<BlobInner>>,
    phantom: PhantomData<T>,
}

impl<T: db::ConnectionProvider> BlobOuter<T> {
    pub(crate) fn new() -> BlobOuter<T> {
        BlobOuter {
            inner: Arc::new(Mutex::new(BlobInner::default())),
            phantom: PhantomData,
        }
    }

    // TODO tests
    pub(crate) fn update(&self) -> Result<()> {
        let mut this = self.inner.lock()?;
        // No point in updating more than once per day.
        if this.last_update.elapsed() < Duration::from_secs(60 * 60 * 24) {
            return Ok(());
        }

        let conn = T::connection()?;
        let mut new_blob = BlobInner::default();
        new_blob.days = db::open_prs_per_day(&conn)?;
        new_blob.weeks = db::weekly_stats(&conn)?;

        *this = new_blob;
        Ok(())
    }
}

#[derive(Clone, Serialize, Debug)]
struct BlobInner {
    #[serde(skip_serializing)]
    last_update: Instant,
    weeks: Vec<Week>,
    days: Vec<Day>,
}

impl Default for BlobInner {
    fn default() -> BlobInner {
        BlobInner {
            last_update: Instant::now(),
            weeks: vec![],
            days: vec![],
        }
    }
}

#[derive(Clone, Serialize, Debug)]
pub struct Week {
    start_date: String,
    pub merged_prs: u32,
    pub closed_prs: u32,
    // In minutes
    pub time_to_merge: Distribution,
    pub review_comments: Distribution,
}

#[derive(Clone, Serialize, Debug)]
pub struct Day {
    pub date: String,
    pub open_prs: u32,
}

#[derive(Clone, Serialize, Debug)]
pub struct Distribution {
    pub mean: u32,
    pub mode: u32,
    pub min: u32,
    pub max: u32,
}
