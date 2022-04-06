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
use std::{fs::DirBuilder, os::unix::fs::DirBuilderExt, process};

#[derive(Parser)]
enum Options {
    Add {
        title: String,
        #[clap(name = "AUTHOR", required = true)]
        authors: Vec<String>,
    },
    Render {
        #[clap(long)]
        complete: bool,
    },
}

fn main() {
    let options = Options::parse();
    let project_dirs = ProjectDirs::from("org.ftbfs", "", "books").unwrap_or_else(|| {
        eprintln!("books: unable to determine home directory");
        process::exit(1);
    });
    let data_dir = project_dirs.data_dir();
    DirBuilder::new()
        .mode(0o700)
        .recursive(true)
        .create(data_dir)
        .unwrap_or_else(|e| {
            eprintln!("books: unable to create {}: {}", data_dir.display(), e);
            process::exit(1);
        });
    let database = data_dir.join("database.sqlite3");
    let mut connection = Connection::open(&database).unwrap_or_else(|e| {
        eprintln!("books: unable to open {}: {}", database.display(), e);
        process::exit(1);
    });
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS book (title TEXT PRIMARY KEY, completion_date TEXT) STRICT; CREATE TABLE IF NOT EXISTS author (title TEXT NOT NULL REFERENCES book (title) ON DELETE CASCADE ON UPDATE CASCADE, author TEXT NOT NULL, PRIMARY KEY (title, author)) STRICT;",
        )
        .unwrap_or_else(|e| {
            eprintln!("books: unable to prepare statement: {}", e);
            process::exit(1);
        });
    connection
        .pragma_update(None, "FOREIGN_KEYS", 1)
        .unwrap_or_else(|e| {
            eprintln!("books: unable to enable foreign key constraints: {}", e);
            process::exit(1);
        });
    match options {
        Options::Add { title, authors } => {
            let transaction = connection.transaction().unwrap_or_else(|e| {
                eprintln!("books: unable to create transaction: {}", e);
                process::exit(1);
            });
            {
                let mut statement = transaction
                    .prepare("INSERT INTO book (title) VALUES (?)")
                    .unwrap_or_else(|e| {
                        eprintln!("books: unable to prepare statement: {}", e);
                        process::exit(1);
                    });
                statement.execute([&title]).unwrap_or_else(|e| {
                    eprintln!("books: unable to execute statement: {}", e);
                    process::exit(1);
                });
            }
            for author in authors {
                let mut statement = transaction
                    .prepare("INSERT INTO author VALUES (?, ?)")
                    .unwrap_or_else(|e| {
                        eprintln!("books: unable to prepare statement: {}", e);
                        process::exit(1);
                    });
                statement.execute([&title, &author]).unwrap_or_else(|e| {
                    eprintln!("books: unable to execute statement: {}", e);
                    process::exit(1);
                });
            }
            transaction.commit().unwrap_or_else(|e| {
                eprintln!("books: unable to commit transaction: {}", e);
                process::exit(1);
            });
        }
        Options::Render { complete } => {
            let statement = if complete {
                "SELECT * FROM book WHERE completion_date IS NOT NULL ORDER BY completion_date DESC"
            } else {
                "SELECT title FROM book WHERE completion_date IS NULL ORDER BY title"
            };
            let mut statement = connection.prepare(statement).unwrap_or_else(|e| {
                eprintln!("books: unable to prepare statement: {}", e);
                process::exit(1);
            });
            let mut rows = statement.query([]).unwrap_or_else(|e| {
                eprintln!("books: unable to execute statement: {}", e);
                process::exit(1);
            });
            while let Some(row) = rows.next().unwrap_or_else(|e| {
                eprintln!("books: unable to execute statement: {}", e);
                process::exit(1);
            }) {
                let title: String = row.get(0).unwrap_or_else(|e| {
                    eprintln!("books: unable to execute statement: {}", e);
                    process::exit(1);
                });
                let mut statement = connection
                    .prepare("SELECT author FROM author WHERE title = ? ORDER BY author")
                    .unwrap_or_else(|e| {
                        eprintln!("books: unable to prepare statement: {}", e);
                        process::exit(1);
                    });
                let rows = statement
                    .query_map([&title], |row| row.get(0))
                    .unwrap_or_else(|e| {
                        eprintln!("books: unable to execute statement: {}", e);
                        process::exit(1);
                    });
                let mut authors = Vec::new();
                for row in rows {
                    let author: String = row.unwrap_or_else(|e| {
                        eprintln!("books: unable to execute statement: {}", e);
                        process::exit(1);
                    });
                    authors.push(author);
                }
                let authors = match authors.len() {
                    1 => authors[0].clone(),
                    2 => format!("{} and {}", authors[0], authors[1]),
                    3 => format!("{}, {}, and {}", authors[0], authors[1], authors[2]),
                    _ => unimplemented!(),
                };
                if complete {
                    let completion_date: String = row.get(1).unwrap_or_else(|e| {
                        eprintln!("books: unable to execute statement: {}", e);
                        process::exit(1);
                    });
                    println!(
                        "      <li><em>{}</em> by {} on {}</li>",
                        title, authors, completion_date
                    );
                } else {
                    println!("      <li><em>{}</em> by {}</li>", title, authors);
                }
            }
        }
    }
}
