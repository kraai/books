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
struct Options {
    title: String,
    #[clap(name = "author", required = true)]
    authors: Vec<String>,
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
        .pragma_update(None, "FOREIGN_KEYS", 1)
        .unwrap_or_else(|e| {
            eprintln!("books: unable to enable foreign key constraints: {}", e);
            process::exit(1);
        });
    let transaction = connection.transaction().unwrap_or_else(|e| {
        eprintln!("books: unable to create transaction: {}", e);
        process::exit(1);
    });
    {
        transaction
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS book (title TEXT PRIMARY KEY, completed TEXT) STRICT; CREATE TABLE IF NOT EXISTS author (title TEXT NOT NULL REFERENCES book (title) ON DELETE CASCADE ON UPDATE CASCADE, author TEXT NOT NULL, PRIMARY KEY (title, author)) STRICT;",
            )
            .unwrap_or_else(|e| {
                eprintln!("books: unable to prepare statement: {}", e);
                process::exit(1);
            });
        let mut statement = transaction
            .prepare("INSERT INTO book (title) VALUES (?)")
            .unwrap_or_else(|e| {
                eprintln!("books: unable to prepare statement: {}", e);
                process::exit(1);
            });
        statement.execute([&options.title]).unwrap_or_else(|e| {
            eprintln!("books: unable to execute statement: {}", e);
            process::exit(1);
        });
    }
    for author in options.authors {
        let mut statement = transaction
            .prepare("INSERT INTO author VALUES (?, ?)")
            .unwrap_or_else(|e| {
                eprintln!("books: unable to prepare statement: {}", e);
                process::exit(1);
            });
        statement
            .execute([&options.title, &author])
            .unwrap_or_else(|e| {
                eprintln!("books: unable to execute statement: {}", e);
                process::exit(1);
            });
    }
    transaction.commit().unwrap_or_else(|e| {
        eprintln!("books: unable to commit transaction: {}", e);
        process::exit(1);
    });
}
