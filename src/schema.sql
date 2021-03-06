-- Copyright 2022 Matthew James Kraai

-- This file is part of books.

-- books is free software: you can redistribute it and/or modify it
-- under the terms of the GNU Affero General Public License as
-- published by the Free Software Foundation, either version 3 of the
-- License, or (at your option) any later version.

-- books is distributed in the hope that it will be useful, but
-- WITHOUT ANY WARRANTY; without even the implied warranty of
-- MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the GNU
-- Affero General Public License for more details.

-- You should have received a copy of the GNU Affero General Public
-- License along with books.  If not, see
-- <https://www.gnu.org/licenses/>.

CREATE TABLE IF NOT EXISTS book (title TEXT PRIMARY KEY, url TEXT, start_date TEXT, end_date TEXT) STRICT;
CREATE TABLE IF NOT EXISTS author (title TEXT NOT NULL REFERENCES book (title) ON DELETE CASCADE ON UPDATE CASCADE, author TEXT NOT NULL, PRIMARY KEY (title, author)) STRICT;
