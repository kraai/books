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
use rusqlite::Connection;
use std::{
    fs::DirBuilder,
    os::unix::fs::DirBuilderExt,
    process::{self, Command},
};

#[derive(Parser)]
enum Options {
    /// Add a book
    Add {
        /// Title of the book
        title: String,
        /// Authors of the book
        #[clap(name = "AUTHOR", required = true)]
        authors: Vec<String>,
    },
    /// Finish reading a book
    Finish {
        /// Title of the book
        title: String,
    },
    /// List books
    #[clap(name = "ls")]
    List,
    /// Change a book's title
    #[clap(name = "mv")]
    Rename {
        /// Old title of the book
        old_title: String,
        /// New title of the book
        new_title: String,
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
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS book (title TEXT PRIMARY KEY, url TEXT, start_date TEXT, end_date TEXT) STRICT; CREATE TABLE IF NOT EXISTS author (title TEXT NOT NULL REFERENCES book (title) ON DELETE CASCADE ON UPDATE CASCADE, author TEXT NOT NULL, PRIMARY KEY (title, author)) STRICT;",
        )
        .unwrap_or_else(|e| die!("cannot execute statement: {}", e));
    connection
        .pragma_update(None, "FOREIGN_KEYS", 1)
        .unwrap_or_else(|e| die!("cannot enable foreign key constraints: {}", e));
    match options {
        Options::Add { title, authors } => {
            let transaction = connection
                .transaction()
                .unwrap_or_else(|e| die!("cannot create transaction: {}", e));
            {
                let mut statement = transaction
                    .prepare("INSERT INTO book (title) VALUES (?)")
                    .unwrap_or_else(|e| die!("cannot prepare statement: {}", e));
                statement
                    .execute([&title])
                    .unwrap_or_else(|e| die!("cannot execute statement: {}", e));
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
        Options::List => {
            let statement = "SELECT title FROM book WHERE start_date is NULL ORDER BY title";
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
