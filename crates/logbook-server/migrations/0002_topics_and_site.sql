-- Spec 4.2, 4.9, 6.5. The owner edits the topics and the title section.
-- Migrations are additive only. Never edit this file after it ships.

-- Topics. The slug is the URL (/topics/<slug>) and never changes. The name and
-- the order can change.
CREATE TABLE topics (
  slug     TEXT PRIMARY KEY,
  name     TEXT NOT NULL,
  position INTEGER NOT NULL
);

INSERT INTO topics (slug, name, position) VALUES
  ('ethereum', 'Ethereum', 1),
  ('rust', 'Rust', 2),
  ('surf', 'Surf', 3),
  ('snowboarding', 'Snowboarding', 4),
  ('jiu-jitsu', 'Jiu jitsu', 5),
  ('classic-wow', 'Classic WoW', 6);

-- 0001 fixed the topics with a CHECK. SQLite cannot drop a CHECK, so the table is
-- built again. Every row and every id stays the same.
CREATE TABLE posts_new (
  id           INTEGER PRIMARY KEY,
  slug         TEXT NOT NULL UNIQUE,
  title        TEXT NOT NULL,
  summary      TEXT NOT NULL DEFAULT '',
  topic        TEXT NOT NULL REFERENCES topics (slug),
  tags         TEXT NOT NULL DEFAULT '[]',
  body_md      TEXT NOT NULL,
  body_html    TEXT NOT NULL,
  word_count   INTEGER NOT NULL,
  state        TEXT NOT NULL DEFAULT 'draft' CHECK (state IN ('draft','private','public')),
  version      INTEGER NOT NULL DEFAULT 1,
  published_at TEXT,
  updated_at   TEXT NOT NULL
);

INSERT INTO posts_new (id, slug, title, summary, topic, tags, body_md, body_html, word_count, state, version, published_at, updated_at)
  SELECT id, slug, title, summary, topic, tags, body_md, body_html, word_count, state, version, published_at, updated_at
  FROM posts;

DROP TABLE posts;
ALTER TABLE posts_new RENAME TO posts;

-- The title section of the home page. One row.
CREATE TABLE site (
  id         INTEGER PRIMARY KEY CHECK (id = 1),
  title      TEXT NOT NULL,
  subtitle   TEXT NOT NULL,
  tagline    TEXT NOT NULL,
  intro_md   TEXT NOT NULL,
  updated_at TEXT
);

INSERT INTO site (id, title, subtitle, tagline, intro_md) VALUES (
  1,
  'Eitan''s Logbook',
  'Software Engineering · Gaming · Random Fun',
  'An Ethereum core developer keeps notes on protocol work, Rust, and the time between.',
  'I work on Ethereum client software, mostly in Rust. Here I write longer notes on protocol changes and performance work. These are the things that do not fit in a call or a GitHub thread.

I also write about surfing, snowboarding, jiu jitsu, and classic WoW.'
);
