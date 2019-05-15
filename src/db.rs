use crate::{Result, DB_PATH};
use crate::data::{self, Date, Sha, Status};

use rusqlite::{self, Connection, Statement,Row, NO_PARAMS, ToSql, params, types::{self, FromSql}};
use std::ops::Range;

/// Create a new database from scratch. Will panic if the db already exists.
pub fn init(conn: &Connection) -> Result<()> {
    data::PullRequest::init(conn)?;
    data::User::init(conn)?;
    data::Sample::init(conn)?;

    // TODO indexes

    Ok(())
}

pub fn connection() -> Result<Connection> {
    Connection::open(DB_PATH).map_err(Into::into)
}

pub fn read_prs(conn: &Connection, _times: Range<Date>) -> Result<Vec<PullRequest>> {
    let reader = Reader::init(conn)?;
    reader.read(_times)
}

// FIXME: we could go further and generate the structs and CREATE statements.
macro_rules! table {
    ($ty: ty, $table: ident, [$($params: ident),*], $create_stmt: expr) => {
        impl $ty {
            pub fn insert_into(&self, conn: &Connection) -> Result<()> {
                conn.execute_named(
                    &format!(
                        "INSERT OR IGNORE INTO {} ({}) VALUES ({})",
                        stringify!($table),
                        stringify!($($params),*),
                        vec![$(format!(":{}", stringify!($params))),*].join(","),
                    ),
                    &[$((&format!(":{}", stringify!($params)), &self.$params as &dyn ToSql)),*],
                )?;
                Ok(())
            }

            fn init(conn: &Connection) -> Result<()> {
                conn.execute($create_stmt, NO_PARAMS)?;
                Ok(())
            }
        }
    }
}

// If the PR/User already exists in the DB then nothing is inserted and the old value
// is kept.
table!(
    data::PullRequest,
    pr,
    [id, number, title, body, author, created, url],
    "CREATE TABLE pr (
        id INTEGER PRIMARY KEY,
        number INTEGER,
        title TEXT NOT NULL,
        body TEXT NOT NULL,
        author INTEGER,
        created TEXT NOT NULL,
        url TEXT NOT NULL
    )"
);
table!(
    data::User,
    user,
    [id, username, url],
    "CREATE TABLE user (
        id INTEGER PRIMARY KEY,
        username TEXT NOT NULL,
        url TEXT NOT NULL
    )"
);
table!(
    data::Sample,
    sample,
    [pr, status, time, commits, additions, deletions, changed_files, review_comments, reviewers, first_commit],
    "CREATE TABLE sample (
        id INTEGER PRIMARY KEY,
        pr INTEGER,
        status TEXT NOT NULL,
        time TEXT NOT NULL,
        commits INTEGER,
        additions INTEGER,
        deletions INTEGER,
        changed_files INTEGER,
        review_comments INTEGER,
        reviewers INTEGER,
        first_commit TEXT NOT NULL
    )"
);


impl ToSql for data::PullRequest {
    fn to_sql(&self) -> rusqlite::Result<types::ToSqlOutput> {
        self.id.to_sql()
    }
}

impl ToSql for data::User {
    fn to_sql(&self) -> rusqlite::Result<types::ToSqlOutput> {
        self.id.to_sql()
    }
}

impl ToSql for Date {
    fn to_sql(&self) -> rusqlite::Result<types::ToSqlOutput> {
        self.date.to_sql()
    }
}

impl FromSql for Date {
    fn column_result(value: types::ValueRef) -> types::FromSqlResult<Self> {
        Ok(Date {
            date: FromSql::column_result(value)?,
        })
    }
}

impl ToSql for Sha {
    fn to_sql(&self) -> rusqlite::Result<types::ToSqlOutput> {
        self.0.to_sql()
    }
}

impl FromSql for Sha {
    fn column_result(value: types::ValueRef) -> types::FromSqlResult<Self> {
        Ok(Sha(FromSql::column_result(value)?))
    }
}

impl ToSql for Status {
    fn to_sql(&self) -> rusqlite::Result<types::ToSqlOutput> {
        match self {
            Status::Open => Ok(types::ToSqlOutput::Owned(types::Value::Text("Open".to_owned()))),
            Status::Closed(d) => Ok(types::ToSqlOutput::Owned(types::Value::Text(format!("Closed {}", d.date)))),
            Status::Merged(d) => Ok(types::ToSqlOutput::Owned(types::Value::Text(format!("Merged {}", d.date)))),
        }
    }
}

impl FromSql for Status {
    fn column_result(value: types::ValueRef) -> types::FromSqlResult<Self> {
        if let types::ValueRef::Text(s) = value {
            match &s[0..1] {
                "O" => return Ok(Status::Open),
                "C" => return Ok(Status::Closed(Date::new(s[7..].to_owned()))),
                "M" => return Ok(Status::Merged(Date::new(s[7..].to_owned()))),
                _ => {}
            }
        }

        Err(types::FromSqlError::InvalidType)
    }
}

struct Reader<'conn> {
    stmt: Statement<'conn>,
    stmt_samples: Statement<'conn>,
}

impl<'conn> Reader<'conn> {
    fn init(conn: &'conn Connection) -> Result<Self> {
        let stmt = conn.prepare(
            "SELECT pr.id, pr.number, pr.title, pr.body, user.username, user.url AS user_url, pr.created, pr.url
                FROM pr, user
                WHERE pr.author = user.id
                ORDER BY pr.number"
        )?;
        let stmt_samples = conn.prepare(
            "SELECT time, status, commits, additions, changed_files, review_comments, reviewers, first_commit
                FROM sample
                WHERE sample.pr = ?1"
        )?;

        Ok(Reader {
            stmt,
            stmt_samples,
        })
    }

    fn read(self, _times: Range<Date>) -> Result<Vec<PullRequest>> {
        let Reader { mut stmt, mut stmt_samples } = self;
        // TODO use range
        let result = Self::collect_query(&mut stmt, NO_PARAMS, |row| {
            let mut pr = PullRequest::from_query(row)?;
            pr.samples = Self::collect_query(&mut stmt_samples, params![row.get::<_, u32>(0)?], Sample::from_query)?;
            pr.author = User::from_query(row)?;
            pr.author.url = row.get("user_url")?;
            Ok(pr)
        })?;

        Ok(result)
    }

    fn collect_query<T>(stmt: &mut Statement<'conn>, params: &[&dyn ToSql], f: impl FnMut(&Row) -> rusqlite::Result<T>) -> rusqlite::Result<Vec<T>> {
        stmt.query_map(params, f)?.collect::<::std::result::Result<Vec<T>, _>>()
    }
}

macro_rules! from_query {
    ($ty: ident, [$($params: ident),*], $($extra: tt)*) => {
        impl $ty {
            fn from_query(row: &Row) -> rusqlite::Result<$ty> {
                Ok($ty {
                    $($params: row.get(stringify!($params))?,)*
                    $($extra)*
                })
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PullRequest {
    pub number: u32,
    pub title: String,
    pub body: String,
    pub author: User,
    pub created: Date,
    pub url: String,
    pub samples: Vec<Sample>,
}

from_query!(PullRequest, [number, title, body, created, url], author: User::default(), samples: vec![]);

#[derive(Debug, Eq, PartialEq, Default)]
pub struct User {
    pub username: String,
    pub url: String,
}

from_query!(User, [username], url: String::new());

#[derive(Debug, Eq, PartialEq)]
pub struct Sample {
    pub time: Date,
    pub status: Status,
    pub commits: u32,
    pub additions: u32,
    pub deletions: u32,
    pub changed_files: u32,
    pub review_comments: u32,
    pub reviewers: u32,
    pub first_commit: Sha,
}

from_query!(Sample, [time, status, commits, additions, deletions, changed_files, review_comments, reviewers, first_commit],);


#[cfg(test)]
mod test {
    use super::*;

    fn init_connection() -> Result<Connection> {
        let conn = Connection::open_in_memory()?;
        init(&conn)?;
        Ok(conn)
    }

    macro_rules! date {
        ($name: ident, $text: expr) => {
            fn $name() -> data::Date {
                data::Date {
                    date: $text.to_owned(),
                }
            }
        }
    }

    macro_rules! pr {
        ($name: ident, $id: expr, $title: expr, $created: expr, $url: expr) => {
            impl data::PullRequest {
                fn $name() -> data::PullRequest {
                    data::PullRequest {
                        id: $id,
                        number: 100 + $id,
                        title: $title.to_owned(),
                        body: format!("Body of {}", $title),
                        author: data::User {
                            id: 42,
                            username: "bob".to_owned(),
                            url: "https://bob".to_owned(),
                        },
                        created: $created,
                        url: $url.to_owned(),
                    }
                }
            }

            impl PullRequest {
                fn $name() -> PullRequest {
                    PullRequest {
                        number: 100 + $id,
                        title: $title.to_owned(),
                        body: format!("Body of {}", $title),
                        author: User {
                            username: "bob".to_owned(),
                            url: "https://bob".to_owned(),
                        },
                        created: $created,
                        url: $url.to_owned(),
                        samples: vec![],
                    }
                }
            }
        }
    }

    date!(date1, "2019-05-15 09:25:34");
    date!(date2, "2019-05-14 09:15:13");
    pr!(pr0, 1, "PR number 0", date1(), "https://pr0");
    pr!(pr1, 2, "PR number 1", date2(), "https://pr1");

    #[test]
    fn insert_and_read() -> Result<()> {
        let conn = init_connection()?;
        let prs = &[data::PullRequest::pr0(), data::PullRequest::pr1()];
        for pr in prs {
            pr.insert_into(&conn)?;
            pr.author.insert_into(&conn)?;
        }
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM pr", NO_PARAMS, |r| r.get::<_, u32>(0))?, 2);
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM user", NO_PARAMS, |r| r.get::<_, u32>(0))?, 1);

        let prs = read_prs(&conn, date1()..date1())?;
        assert_eq!(prs.len(), 2);

        assert_eq!(prs[0], PullRequest::pr0());
        assert_eq!(prs[1], PullRequest::pr1());
        Ok(())
    }
}
