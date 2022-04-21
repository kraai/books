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
use time::{format_description, Date};

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
    /// Read a book
    Read {
        /// Title of the book
        title: String,
    },
    /// Change a book's title
    #[clap(name = "mv")]
    Rename {
        /// Old title of the book
        old_title: String,
        /// New title of the book
        new_title: String,
    },
    /// Render the list of books
    Render {
        /// Render completed books
        #[clap(long)]
        complete: bool,
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
            "CREATE TABLE IF NOT EXISTS book (title TEXT PRIMARY KEY, completion_date TEXT) STRICT; CREATE TABLE IF NOT EXISTS author (title TEXT NOT NULL REFERENCES book (title) ON DELETE CASCADE ON UPDATE CASCADE, author TEXT NOT NULL, PRIMARY KEY (title, author)) STRICT;",
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
            update_website();
        }
        Options::Read { title } => {
            if connection
                .execute(
                    "UPDATE book SET completion_date = date('now','localtime') WHERE title = ?",
                    [&title],
                )
                .unwrap_or_else(|e| die!("cannot execute statement: {}", e))
                != 1
            {
                die!("not found: {}", title);
            }
            update_website();
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
            update_website();
        }
        Options::Render { complete } => {
            let statement = if complete {
                "SELECT * FROM book WHERE completion_date IS NOT NULL ORDER BY completion_date DESC"
            } else {
                "SELECT title FROM book WHERE completion_date IS NULL ORDER BY title"
            };
            let mut statement = connection
                .prepare(statement)
                .unwrap_or_else(|e| die!("cannot prepare statement: {}", e));
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
                let mut statement = connection
                    .prepare("SELECT author FROM author WHERE title = ? ORDER BY author")
                    .unwrap_or_else(|e| die!("cannot prepare statement: {}", e));
                let rows = statement
                    .query_map([&title], |row| row.get(0))
                    .unwrap_or_else(|e| die!("cannot execute statement: {}", e));
                let mut authors = Vec::new();
                for row in rows {
                    let author: String =
                        row.unwrap_or_else(|e| die!("cannot execute statement: {}", e));
                    authors.push(author);
                }
                let authors = match authors.len() {
                    1 => authors[0].clone(),
                    2 => format!("{} and {}", authors[0], authors[1]),
                    3 => format!("{}, {}, and {}", authors[0], authors[1], authors[2]),
                    _ => unimplemented!(),
                };
                if complete {
                    let completion_date: String = row
                        .get(1)
                        .unwrap_or_else(|e| die!("cannot execute statement: {}", e));
                    let completion_date = Date::parse(
                        &completion_date,
                        &format_description::parse("[year]-[month]-[day]").unwrap(),
                    )
                    .unwrap_or_else(|e| die!("cannot parse completion date for {}: {}", title, e));
                    println!(
                        "      <li><em>{}</em> by {}, finished on {}</li>",
                        title,
                        authors,
                        completion_date
                            .format(
                                &format_description::parse(
                                    "[month repr:long] [day padding:none], [year padding:none]"
                                )
                                .unwrap()
                            )
                            .unwrap_or_else(|e| die!(
                                "cannot format completion date for {}: {}",
                                title,
                                e
                            ))
                    );
                } else {
                    println!("      <li><em>{}</em> by {}</li>", title, authors);
                }
            }
        }
    }
}

fn update_website() {
    if !Command::new("make")
        .args(["-C", "/home/kraai/src/ftbfs.org"])
        .status()
        .unwrap_or_else(|e| die!("cannot run make: {}", e))
        .success()
    {
        die!("make failed");
    }
}
