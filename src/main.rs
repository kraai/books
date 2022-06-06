// Copyright 2022 Matthew James Kraai

// This file is part of books.

// books is free software: you can redistribute it and/or modify it
// under the terms of the GNU Affero General Public License as
// published by the Free Software Foundation, either version 3 of the
// License, or (at your option) any later version.

// books is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
// Affero General Public License for more details.

// You should have received a copy of the GNU Affero General Public
// License along with books.  If not, see
// <https://www.gnu.org/licenses/>.

use clap::Parser;
use directories::ProjectDirs;
use pager::Pager;
use rusqlite::{Connection, OptionalExtension};
use std::{fs::DirBuilder, os::unix::fs::DirBuilderExt, process};

#[derive(Parser)]
enum Options {
    /// Add a book
    Add {
        /// Title of the book
        title: String,
        /// Authors of the book
        #[clap(name = "AUTHOR", required = true)]
        authors: Vec<String>,
        /// URL of the book
        #[clap(long)]
        url: Option<String>,
    },
    /// Finish reading a book
    Finish {
        /// Title of the book
        title: String,
    },
    /// List books
    #[clap(name = "ls")]
    List {
        /// List finished books instead of unstarted ones
        #[clap(long)]
        finished: bool,
        /// List started books instead of unstarted ones
        #[clap(long)]
        started: bool,
        /// List books with no URL
        #[clap(long)]
        without_url: bool,
    },
    /// Change a book's title
    #[clap(name = "mv")]
    Rename {
        /// Old title of the book
        old_title: String,
        /// New title of the book
        new_title: String,
    },
    /// Set a book's URL
    #[clap(name = "set-url")]
    SetUrl {
        /// Title of the book
        title: String,
        /// URL of the book
        url: String,
    },
    Show {
        /// Title of the book
        title: String,
    },
    /// Start reading a book
    Start {
        /// Title of the book
        title: String,
    },
}

macro_rules! die {
    ($fmt:expr) => ({
	eprintln!(concat!("books: ", $fmt));
	process::exit(1);
    });
    ($fmt:expr, $($arg:tt)*) => ({
	eprintln!(concat!("books: ", $fmt), $($arg)*);
	process::exit(1);
    });
}

fn main() {
    let options = Options::parse();
    let project_dirs = ProjectDirs::from("org.ftbfs", "", "books")
        .unwrap_or_else(|| die!("cannot determine home directory"));
    let data_dir = project_dirs.data_dir();
    DirBuilder::new()
        .mode(0o700)
        .recursive(true)
        .create(data_dir)
        .unwrap_or_else(|e| die!("cannot create {}: {}", data_dir.display(), e));
    let database = data_dir.join("database.sqlite3");
    let mut connection = Connection::open(&database)
        .unwrap_or_else(|e| die!("cannot open {}: {}", database.display(), e));
    connection
        .execute_batch(include_str!("schema.sql"))
        .unwrap_or_else(|e| die!("cannot execute statement: {}", e));
    connection
        .pragma_update(None, "FOREIGN_KEYS", 1)
        .unwrap_or_else(|e| die!("cannot enable foreign key constraints: {}", e));
    match options {
        Options::Add {
            title,
            authors,
            url,
        } => {
            let transaction = connection
                .transaction()
                .unwrap_or_else(|e| die!("cannot create transaction: {}", e));
            {
                if let Some(url) = url {
                    let mut statement = transaction
                        .prepare("INSERT INTO book (title, url) VALUES (?, ?)")
                        .unwrap_or_else(|e| die!("cannot prepare statement: {}", e));
                    statement
                        .execute([&title, &url])
                        .unwrap_or_else(|e| die!("cannot execute statement: {}", e));
                } else {
                    let mut statement = transaction
                        .prepare("INSERT INTO book (title) VALUES (?)")
                        .unwrap_or_else(|e| die!("cannot prepare statement: {}", e));
                    statement
                        .execute([&title])
                        .unwrap_or_else(|e| die!("cannot execute statement: {}", e));
                }
            }
            for author in authors {
                let mut statement = transaction
                    .prepare("INSERT INTO author VALUES (?, ?)")
                    .unwrap_or_else(|e| die!("cannot prepare statement: {}", e));
                statement
                    .execute([&title, &author])
                    .unwrap_or_else(|e| die!("cannot execute statement: {}", e));
            }
            transaction
                .commit()
                .unwrap_or_else(|e| die!("cannot commit transaction: {}", e));
        }
        Options::Finish { title } => {
            if connection
                .execute(
                    "UPDATE book SET end_date = date('now','localtime') WHERE title = ?",
                    [&title],
                )
                .unwrap_or_else(|e| die!("cannot execute statement: {}", e))
                != 1
            {
                die!("not found: {}", title);
            }
        }
        Options::List {
            finished,
            started,
            without_url,
        } => {
            Pager::new().setup();
            let statement = if finished {
                "SELECT title FROM book WHERE end_date IS NOT NULL ORDER BY end_date"
            } else if started {
                "SELECT title FROM book WHERE start_date IS NOT NULL AND end_date IS NULL ORDER BY title"
            } else if without_url {
                "SELECT title FROM book WHERE url IS NULL ORDER BY title"
            } else {
                "SELECT title FROM book WHERE start_date IS NULL ORDER BY title"
            };
            let mut statement = connection
                .prepare(statement)
                .unwrap_or_else(|e| die!("cannot prepare statement \"{}\": {}", statement, e));
            let mut rows = statement
                .query([])
                .unwrap_or_else(|e| die!("cannot execute statement: {}", e));
            while let Some(row) = rows
                .next()
                .unwrap_or_else(|e| die!("cannot execute statement: {}", e))
            {
                let title: String = row
                    .get(0)
                    .unwrap_or_else(|e| die!("cannot execute statement: {}", e));
                println!("{}", title);
            }
        }
        Options::Rename {
            old_title,
            new_title,
        } => {
            if connection
                .execute(
                    "UPDATE book SET title = ? WHERE title = ?",
                    [&new_title, &old_title],
                )
                .unwrap_or_else(|e| die!("cannot execute statement: {}", e))
                != 1
            {
                die!("not found: {}", old_title);
            }
        }
        Options::SetUrl { title, url } => {
            if connection
                .execute("UPDATE book SET url = ? WHERE title = ?", [&url, &title])
                .unwrap_or_else(|e| die!("cannot execute statement: {}", e))
                != 1
            {
                die!("not found: {}", title);
            }
        }
        Options::Show { title } => {
            Pager::new().setup();
            if let Some((url, start_date, end_date)) = connection
                .query_row(
                    "SELECT url, start_date, end_date FROM book WHERE title = ?",
                    [&title],
                    |row| {
                        row.get(0).and_then(|url: Option<String>| {
                            row.get(1).and_then(|start_date: Option<String>| {
                                row.get(2).and_then(|end_date: Option<String>| {
                                    Ok((url, start_date, end_date))
                                })
                            })
                        })
                    },
                )
                .optional()
                .unwrap_or_else(|e| die!("cannot execute statement: {}", e))
            {
                println!("Title: {}", title);
                if let Some(url) = url {
                    println!("URL: {}", url);
                }
                if let Some(start_date) = start_date {
                    println!("Started: {}", start_date);
                }
                if let Some(end_date) = end_date {
                    println!("Finished: {}", end_date);
                }
                let mut authors = Vec::new();
                let mut statement = connection
                    .prepare("SELECT author FROM author WHERE title = ?")
                    .unwrap_or_else(|e| die!("cannot prepare statement: {}", e));
                let mut rows = statement
                    .query([&title])
                    .unwrap_or_else(|e| die!("cannot execute statement: {}", e));
                while let Some(row) = rows
                    .next()
                    .unwrap_or_else(|e| die!("cannot execute statement: {}", e))
                {
                    let author: String = row
                        .get(0)
                        .unwrap_or_else(|e| die!("cannot execute statement: {}", e));
                    authors.push(author);
                }
                println!("Authors: {}", authors.join(", "));
            }
        }
        Options::Start { title } => {
            if connection
                .execute(
                    "UPDATE book SET start_date = date('now','localtime') WHERE title = ?",
                    [&title],
                )
                .unwrap_or_else(|e| die!("cannot execute statement: {}", e))
                != 1
            {
                die!("not found: {}", title);
            }
        }
    }
}
