-- Spec 6.5. Migrations are additive only. Never edit this file after it ships.

CREATE TABLE posts (
  id           INTEGER PRIMARY KEY,
  slug         TEXT NOT NULL UNIQUE,
  title        TEXT NOT NULL,
  summary      TEXT NOT NULL DEFAULT '',
  topic        TEXT NOT NULL CHECK (topic IN ('ethereum','rust','surf','snowboarding','jiu-jitsu','classic-wow')),
  tags         TEXT NOT NULL DEFAULT '[]',
  body_md      TEXT NOT NULL,
  body_html    TEXT NOT NULL,
  word_count   INTEGER NOT NULL,
  state        TEXT NOT NULL DEFAULT 'draft' CHECK (state IN ('draft','private','public')),
  version      INTEGER NOT NULL DEFAULT 1,
  published_at TEXT,
  updated_at   TEXT NOT NULL
);

CREATE TABLE now_box (
  id         INTEGER PRIMARY KEY CHECK (id = 1),
  body_md    TEXT NOT NULL,
  body_html  TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE passkeys (
  id           BLOB PRIMARY KEY,
  passkey      TEXT NOT NULL,
  label        TEXT NOT NULL,
  created_at   TEXT NOT NULL,
  last_used_at TEXT
);

CREATE TABLE sessions (
  token_hash BLOB PRIMARY KEY,
  created_at TEXT NOT NULL,
  expires_at TEXT NOT NULL
);

CREATE TABLE setup_tokens (
  token_hash BLOB PRIMARY KEY,
  expires_at TEXT NOT NULL,
  used_at    TEXT
);

CREATE TABLE visits (
  day   TEXT PRIMARY KEY,
  count INTEGER NOT NULL
);

CREATE TABLE heartbeat (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  at TEXT NOT NULL
);
