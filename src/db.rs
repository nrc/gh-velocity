use crate::{Result, DB_PATH};
use crate::data::{self, Date};

use rusqlite::{Connection, NO_PARAMS, ToSql, params};
use std::ops::Range;

/// Create a new database from scratch. Will panic if the db already exists.
pub fn init(conn: &Connection) -> Result<()> {
    // TODO full tables, foreign keys
    conn.execute("CREATE TABLE pr (
            id INTEGER PRIMARY KEY,
            number INTEGER,
            title TEXT NOT NULL,
            author INTEGER
        )", NO_PARAMS)?;
    conn.execute("CREATE TABLE user (
            id INTEGER PRIMARY KEY,
            username TEXT NOT NULL
        )", NO_PARAMS)?;
    conn.execute("CREATE TABLE sample (
            id INTEGER PRIMARY KEY,
            pr INTEGER,
            time TEXT NOT NULL
        )", NO_PARAMS)?;
    // TODO indexes

    Ok(())
}

macro_rules! insert {
    ($ty: ty, $sql: expr) => {
        impl $ty {
            pub fn insert_into(&self, conn: &Connection) -> Result<()> {
                self.with_params(|params| {
                    // TODO use prepared statements
                    conn.execute($sql, params)
                })?;
                Ok(())
            }
        }
    }
}

// If the PR/User already exists in the DB then nothing is inserted and the old value
// is kept.
insert!(data::PullRequest, "INSERT OR IGNORE INTO pr (id, number, title, author) values (?1, ?2, ?3, ?4)");
insert!(data::User, "INSERT OR IGNORE INTO user (id, username) values (?1, ?2)");
insert!(data::Sample, "INSERT INTO sample (pr, time) values (?1, ?2)");

pub fn connection() -> Result<Connection> {
    Connection::open(DB_PATH).map_err(Into::into)
}

trait Params {
    fn with_params<F, T>(&self, f: F) -> T where F: FnOnce(&[&dyn ToSql]) -> T;
}

impl Params for data::PullRequest {
    fn with_params<F, T>(&self, f: F) -> T where F: FnOnce(&[&dyn ToSql]) -> T {
        f(params![self.id, self.number, self.title, self.author.id])
    }
}

impl Params for data::User {
    fn with_params<F, T>(&self, f: F) -> T where F: FnOnce(&[&dyn ToSql]) -> T {
        f(params![self.id, self.username])
    }
}

impl Params for data::Sample {
    fn with_params<F, T>(&self, f: F) -> T where F: FnOnce(&[&dyn ToSql]) -> T {
        f(params![self.pr.id, self.time.param_str()])
    }
}

pub fn read_prs(conn: &Connection, _times: Range<Date>) -> Result<Vec<PullRequest>> {
    // TODO range
    let mut stmt = conn.prepare("SELECT pr.id, pr.number, pr.title, user.username FROM pr, user WHERE pr.author = user.id ORDER BY pr.number")?;
    let mut stmt_samples = conn.prepare("SELECT sample.time FROM sample WHERE sample.pr = ?1")?;

    let result = stmt.query_map(NO_PARAMS, |row| {
        let samples = stmt_samples.query_map(params![row.get::<_, u32>(0)?], |row| {
            Ok(Sample {
                time: Date::new(row.get(0)?),
            })
        })?.collect::<::std::result::Result<Vec<_>, _>>()?;
        Ok(PullRequest {
            number: row.get(1)?,
            title: row.get(2)?,
            author: User {
                username: row.get(3)?,
            },
            samples,
        })
    })?.collect::<::std::result::Result<Vec<_>, _>>().map_err(Into::into);

    result
}

#[derive(Debug, Eq, PartialEq)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub author: User,
    pub samples: Vec<Sample>,
}

#[derive(Debug, Eq, PartialEq)]
pub struct User {
    pub username: String,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Sample {
    pub time: Date,
}

#[cfg(test)]
mod test {
    use super::*;

    fn init_connection() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        init(&conn)?;
        Ok(conn)
    }

    fn date1() -> data::Date {
        data::Date {
            date: "2019-05-15 09:25:34".to_owned(),
        }
    }

    fn date2() -> data::Date {
        data::Date {
            date: "2019-05-14 09:15:13".to_owned(),
        }
    }

    fn pr1() -> data::PullRequest {
        data::PullRequest {
            id: 1,
            number: 101,
            title: "PR number 1".to_owned(),
            body: "Body 1".to_owned(),
            author: data::User {
                id: 42,
                username: "bob".to_owned(),
                url: "https://bob".to_owned(),
            },
            created: date1(),
            url: "https://pr1".to_owned(),
        }
    }

    fn pr2() -> data::PullRequest {
        data::PullRequest {
            id: 2,
            number: 105,
            title: "PR number 2".to_owned(),
            body: "Body 2".to_owned(),
            author: data::User {
                id: 42,
                username: "bob".to_owned(),
                url: "https://bob".to_owned(),
            },
            created: date2(),
            url: "https://pr2".to_owned(),
        }
    }

    #[test]
    fn insert_and_read() -> Result<()> {
        let conn = init_connection()?;
        let prs = &[pr1(), pr2()];
        for pr in prs {
            pr.insert_into(&conn);
            pr.author.insert_into(&conn);
        }
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM pr", NO_PARAMS, |r| r.get::<_, u32>(0))?, 2);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM user", NO_PARAMS, |r| r.get::<_, u32>(0))?, 1);

        let prs = read_prs(&conn, date1()..date1())?;
        assert_eq!(prs.len(), 2);

        assert_eq!(prs[0], PullRequest {
            number: 101,
            title: "PR number 1".to_owned(),
            author: User {
                username: "bob".to_owned(),
            },
            samples: vec![],
        });
        assert_eq!(prs[1], PullRequest {
            number: 105,
            title: "PR number 2".to_owned(),
            author: User {
                username: "bob".to_owned(),
            },
            samples: vec![],
        });
        Ok(())
    }
}
