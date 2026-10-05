# Eitan's Logbook: spec

Status: design done, build not started. Updated 2026-10-04.

Personal blog. One author: Eitan. Visitors read. Only Eitan writes.

Notation: **[decided]** = agreed. **[verified 2026-10-04]** = checked against docs or a live request. **[build-check]** = checked with real code during the build.

## 1. Goals

- 1998 desktop look. Easy to read.
- Markdown posts. Split editor: markdown left, live preview right.
- Three post states: draft, private, public.
- Native web components. No framework.
- The site comes back on a new server in about 10 minutes, from one bucket and one secret.
- High test bar, with formal proofs of the access rules.

## 2. Files

| Path | Content |
|---|---|
| `designs/logbook.html` | Chosen design, working mockup |
| `designs/components.js` | Mockup components and sample data |

Repo: https://github.com/eserilev/blog (public). Default branch: `master`.

Mockup online: https://claude.ai/artifact/LmzY8m7y3TToGKU9RzwLqQ

The mockup is a design reference only. It uses inline scripts, Google Fonts, hash views, and unescaped `innerHTML`. The real build has none of these (sections 5, 6.9).

## 3. Design

### 3.1 Concept

A Windows 98 desktop. One browser window holds the site. The chrome carries the 90s look. The page inside stays quiet.

Chrome, all kept:

- Title bar: minimize, maximize, close.
- Menu bar: File, Edit, View, Go, Bookmarks, Help. Decoration only.
- Toolbar: Back, Home, Latest, Compose (owner only), Print.
- Location bar: shows a path, for example `file:///home/eitan/www/index.html`.
- Status bar.
- Taskbar: Start, one button per view, clock.
- Teal desktop.

No "Netscape" text anywhere. The window title is the page name.

### 3.2 Logo

- Black square, 46 × 46 px, toolbar top right.
- White serif "E" over two waves (white front, gray back).
- Waves move on hover and for 1.4 s after each view change. No motion with `prefers-reduced-motion`.
- 16 px version in the title bar.
- Status: first draft.

### 3.3 Header

- Title: **Eitan's Logbook** by default. Tinos bold, 44–68 px, raised gray shadow.
- Under it, the subtitle: "SOFTWARE ENGINEERING · GAMING · RANDOM FUN" by default. Spaced monospace capitals, navy.
- 6 px navy-to-blue bar, same gradient as the title bar.
- Tagline: "An Ethereum core developer keeps notes on protocol work, Rust, and the time between." by default.
- The owner edits the title, subtitle, tagline, and intro (4.9).

### 3.4 Colors

| Token | Value | Use |
|---|---|---|
| `--desk` | `#008080` | Desktop |
| `--chrome` | `#c0c0c0` | Window chrome, buttons |
| `--title-a` → `--title-b` | `#000080` → `#1084d0` | Title bars, header bar |
| `--page` | `#ffffff` | Page |
| `--text` | `#1a1a1a` | Text, headings |
| `--muted` | `#5c5c5c` | Dates, summaries |
| `--rule` | `#c8c8c8` | Lines |
| `--link` | `#0000cc` | Links, underlined |
| `--navy` | `#000060` | Sidebar. The only accent. |
| LCD | `#38ff38` on `#0a1a0a` | Counter and surf report only |

Single light theme, by choice.

### 3.5 Type

All fonts are self-hosted in `static/fonts/`. No request goes to Google.

| Role | Font | Fallback |
|---|---|---|
| Headings | Tinos | Times New Roman, serif |
| Body | Verdana | DejaVu Sans, Geneva, sans-serif |
| Chrome | Tahoma | Segoe UI, DejaVu Sans, sans-serif |
| Code, dates, paths | Cousine | Courier New, monospace |
| LCD | VT323 | Courier New, monospace |

Body 15–15.5 px, line height 1.65–1.75, max about 68 characters per line.

### 3.6 Home page

Sidebar (navy):

1. Site: Home, Latest post, Compose (owner only), Edit site (owner only), About, RSS feed.
2. Topics: the owner's list (4.2). The server writes the links into the page.
3. Find me: GitHub, X, Discord, with small white icons.
4. Visitors: green LCD counter.

Main column:

1. Header (3.3).
2. Intro: markdown that the owner edits (4.9).
3. Writing: table of public posts. Columns: Date (M/D/YY), Title + one-line summary, Topic, Length. Small navy "NEW" label on new posts. No blink.
4. Now box (4.4) and Surf Report window (4.5), side by side.
5. Footer: last updated, "Best viewed at 800 × 600", Sign in / Sign out.

Below 860 px: the sidebar goes below the content, so the posts come first. Editor panes stack.

### 3.7 Post page

- Breadcrumbs: Home › Topic.
- Title (Tinos), date, reading time.
- Thin black rules above and below the body.
- Code blocks: black, light gray text, first line `C:\> type example.rust`.
- Quotes: gray left border, gray italic.

### 3.8 Editor (owner only)

- Left window: "<slug>.md - Notepad". Markdown textarea. Toolbar: Bold, Italic, H2, List, Code, Link.
- Right window: "Preview". Status line: "Document: Done · N words · M min read".
- `*` in the Notepad title when unsaved.
- Preview scroll follows the textarea. Tab inserts four spaces.
- Publish dialog: state choice (Draft, Private, Public). Buttons: Save, Publish.
- Result message box:
  - Public: "Post published. Anyone can read it now."
  - Private: "Saved as private. Only you can see this post."
  - Draft: "Draft saved."
  - Conflict: "This post changed on another device. Reload to see the newer version."

### 3.9 Accessibility

- Every control works with the keyboard. Focus is always visible.
- Decorative chrome (menu bar, disabled buttons, window controls) has `aria-hidden="true"` and no tab stop.
- Real landmarks: `header`, `nav`, `main`, `footer`.
- Contrast: body text and links meet WCAG AA.
- Message boxes use `role="status"`.

## 4. Features

### 4.1 Post states [decided]

| State | Who sees it | In RSS and export | Notes |
|---|---|---|---|
| `draft` | Owner | No | Work in progress |
| `private` | Owner | No | Finished, kept to self |
| `public` | Everyone | Yes | Published |

One field, `state`. No other flag controls visibility.

`published_at` is set on the first change to `public`. It does not change later.

### 4.2 Post fields

| Field | Type | Notes |
|---|---|---|
| `id` | integer (u64) | Stable |
| `slug` | string | Set on create. Never changes (4.3). |
| `title` | string | |
| `summary` | string | One line |
| `topic` | string | Topic slug. Must exist in `topics` (see below). |
| `tags` | string[] | |
| `body_md` | markdown | |
| `body_html` | HTML | Rendered and sanitized at save time |
| `word_count` | integer | Server computes |
| `state` | enum | 4.1 |
| `version` | integer | Goes up by 1 on each save (6.4) |
| `published_at` | timestamp or null | |
| `updated_at` | timestamp | |

Reading time: `max(1, ceil(words / 220))` minutes.

**Topics.** The owner manages the list in Compose → Topics. Defaults: Ethereum, Rust, Surf, Snowboarding, Jiu jitsu, Classic WoW.

- Each topic has a slug, a name, and a position. The slug comes from the name when the topic is made (`make_slug`). The URL `/topics/<slug>` never changes, so a rename keeps old links working.
- Add: a name of 1–40 characters with at least one ASCII letter or digit. A slug that exists → 409. At most 30 topics.
- Rename and order: one save with every topic once. Any other set → 400.
- Delete: refused (409) while posts use the topic, and for the last topic.
- A new post gets the first topic.

### 4.3 Slugs

- Follows the title while the post has never been public. Frozen on the first publish: a later title change does not change it. Old links never break, because no public link exists before the first publish.
- Rules: `a-z`, `0-9`, `-`. 1–80 bytes. No `-` at either end. No `--`.
- A title with no ASCII letters or digits (for example, Hebrew) gets `post-<id>`.
- If the slug is already taken, the new post gets `post-<id>`.
- An apostrophe is dropped, not turned into `-` (`client's` → `clients`).

### 4.4 Now box

- Short markdown list, at most 5,000 characters. Owner edits in place (Edit, Save, Cancel). One owner, so the last save wins (no version check).
- Shows "Updated <date>".
- Content: placeholder until Eitan supplies it.

### 4.5 Surf report [decided]

One spot: **Redondo Beach, CA**. All data from NOAA. Public domain. No API key.

Win98 window "Surf Report - Redondo Beach". LCD readouts and a tide chart. No rating text.

| Readout | Source | Endpoint | Unit shown |
|---|---|---|---|
| Swell height, period, direction | NDBC buoy **46221**, Santa Monica Bay (33.860 N, 118.641 W) | `https://www.ndbc.noaa.gov/data/realtime2/46221.spec` (`SwH`, `SwP`, `SwD`) | ft, s, compass |
| Combined wave height | same buoy | `.../46221.txt` (`WVHT`) | ft |
| Water temperature | same buoy | `.../46221.txt` (`WTMP`) | °F and °C |
| Wind speed, direction | NWS station **KTOA**, Torrance airport (about 4 mi inland) | `https://api.weather.gov/stations/KTOA/observations/latest` | kt, compass |
| Tide highs and lows | NOAA CO-OPS station **9410738**, King Harbor | `https://api.tidesandcurrents.noaa.gov/api/prod/datagetter?product=predictions&station=9410738&interval=hilo&datum=MLLW&units=english&time_zone=lst_ldt&format=json&application=logbook` | ft, local time |

[verified 2026-10-04] by live requests:

- 46221 is a wave buoy. It has **no wind data** (`MM`). Wind comes from KTOA.
- 9410738 gives **high and low predictions only**. `interval=h` returns an error. The chart draws a cosine curve between the highs and lows. This is the standard method for this kind of station.
- The NWS API needs a `User-Agent` header with contact info. NWS gives wind in km/h. The server converts it to knots.
- NDBC files are plain text, newest row first. `MM` = missing. Units are metric (m, m/s, °C). The server converts them.

Rules:

- The server fetches every 30 min and caches. The page calls only `/api/surf`.
- Each readout shows its own observation time. A readout older than 3 h is grayed out with "stale".
- If one source fails, the others still show.
- Attribution line: "Data: NOAA NDBC, NWS, CO-OPS".
- Rating, in `surf.rs` with unit tests: swell under 1 ft → "Flat."; onshore wind of 12 kt or more → "Blown out."; under 2 ft → "Small."; period of 12 s or more with offshore or light (< 6 kt) wind → "Good."; else "Fair."
- Offshore for Redondo = wind from 30° to 150° (the land side).
- Tides: `begin_date` = yesterday (UTC), `range=72`. (`range` alone counts back from now, not ahead.)
- A failed source keeps its last good value. `LOGBOOK_SURF=off` turns the fetch off (tests).
- Parser tests use real responses saved in `tests/fixtures/noaa/`, including a station report with `null` wind.

### 4.6 Owner access

In the page:

- Guests see no Compose, no Notepad, no Now Edit button.
- `/write` shows nothing without a session.
- Footer "Sign in" opens a Win98 Sign In dialog. Fixed user name `eitan`.
- Signed in: editor controls appear, status bar shows "Signed in as eitan".

On the server (the real protection): section 6.6.

### 4.7 RSS

- `/feed.xml`, public posts only.
- `guid` = `logbook-post-<id>`, `isPermaLink="false"`.
- Item date = `published_at`.
- Absolute URLs for links and `/media/` images.
- Built on each request from the current public posts, so it is never stale. `Cache-Control: public, max-age=300`.
- Each item has the summary as `description` and the full HTML as `content:encoded`.
- `<link rel="alternate" type="application/rss+xml">` in `index.html`.
- Status: proposed. Eitan has not decided.

### 4.8 Visitor counter

- Counted in memory. No cookies. No IPs stored.
- A visit = a page load (`/`, `/posts/*`, `/topics/*`, and the other page routes) with a user agent that is not empty and does not look like a bot or tool (`bot`, `crawl`, `spider`, `curl`, `wget`, `python`, `feed`, `headless`, and similar). API calls do not count.
- Flushed to SQLite once per minute.
- Open question: real count or fixed number.

### 4.9 Title section

The owner edits the title section in Compose → Site Settings. One row in `site`.

| Field | Limit | Used in |
|---|---|---|
| `title` | 1–80 characters | Masthead, window title, `<title>`, `og:site_name`, RSS title |
| `subtitle` | 0–120 | Masthead |
| `tagline` | 0–300 | Masthead, default `<meta name="description">`, RSS description |
| `intro_md` | 0–5,000, markdown | Home page intro, through the markdown pipeline (6.7) |

- The server fills these values and the topic list into `index.html` on each page load. Text is escaped. So the page needs no extra request and has no flash of old text.
- One owner, so the last save wins.

## 5. Front end

Native custom elements, light DOM.

| Element | Job |
|---|---|
| `<blog-app>` | Path router (History API). Maps `/`, `/posts/{slug}`, `/topics/{topic}`, `/about`, `/write`, `/write/{id}` to views. Intercepts same-origin link clicks. Fires `viewchange`. |
| `<post-list>` | Fetches a list API. Stamps its `<template>` per post, with `textContent` only. |
| `<post-view>` | Fetches one post. Puts `body_html` into the page. |
| `<now-box>` | Shows the Now box. Owner: edit and save. |
| `<surf-report>` | Fetches `/api/surf`. |
| `<md-editor>` | Textarea, WASM preview, toolbar, word count, state choice, Save and Publish, message box. |
| `<sign-in>` | Passkey dialog. |

Rules:

- **Only `body_html` goes into `innerHTML`.** It is sanitized on the server. Everything else uses `textContent` or `setAttribute`.
- No inline scripts or inline event handlers (CSP, 6.9).
- `GET /api/me` decides if owner controls show. Hidden controls are not security.

Preview: the same Rust `render()` as the server (comrak, syntect, ammonia), compiled to WASM. Loaded on `/write` only. Its output is sanitized, so it is safe in `innerHTML`. syntect uses the `fancy-regex` feature on both sides, so outputs match. [build-check done] Size: 2.8 MB, 1.1 MB gzipped (mostly syntect's syntax data). The first render takes about 80 ms; later renders about 1 ms per post. Only the owner loads it.

The WASM interface is three exported functions (`buf_alloc`, `buf_free`, `render`), no bindings generator. This small shim in `editor-wasm` is the only `unsafe` code in the project (7.1).

## 6. Back end

Status: proposed. Not built.

### 6.1 Jobs

- Store posts, the Now box, passkeys, sessions, counts.
- JSON API for the components. Serve `index.html` and static files.
- Sign in the owner. Check the session on every owner request.
- Never send a non-public post to a guest.
- Fetch and cache surf data.
- Write RSS. Export public posts to git. Build the owner zip.

The server does not build page layout (option A, [decided]).

### 6.2 Stack

| Part | Choice |
|---|---|
| Language | Rust (stable; Charon needs a pinned nightly, proofs only) |
| Web | axum, tokio |
| DB | SQLite, sqlx. WAL mode. |
| Markdown | comrak → syntect (`fancy-regex`) → ammonia |
| Passkeys | webauthn-rs 0.5 (needs OpenSSL at build and run time) |
| Sessions | Own small table, token hashes only (6.6) |
| S3 | aws-sdk-s3 or object_store, virtual-host addressing |
| Backups | Litestream, pinned version |
| HTTP client | reqwest |
| RSS | rss crate |
| TLS | sandcastle's Caddy (6.11) |

Output: one binary, one SQLite file, one static folder. No template library.

### 6.3 Page delivery

1. Request `/posts/{slug}`.
2. The server looks up the slug **through `reveal`** (7.6).
   - Public: serve `index.html` with the title and summary in `<title>`, `og:*`, and `twitter:*` tags. Values are HTML-escaped.
   - Draft, private, or unknown: serve `index.html` with status **404**, a generic title, and `<meta name="robots" content="noindex">`. No post data.
3. The browser loads the CSS and `components.js`.
4. `<post-view>` calls `GET /api/posts/{slug}` and shows `body_html`.

The same rule (real status, no leak) holds for `/topics/{topic}`: unknown topic → 404.

No JavaScript → a short message in the window. [decided]

### 6.4 Routes

All routes live in one table in code (`routes.rs`): path, method, access level, handler. The router and the access matrix test (7.4) are both built from this table. axum has no route listing, so this table is the source of truth. The test `every_kind_is_routed` fails if a handler kind has no row in the table.

Pages (return `index.html`):

| Path | Notes |
|---|---|
| `/`, `/about` | |
| `/posts/{slug}` | 6.3 |
| `/topics/{topic}` | |
| `/write`, `/write/{id}` | Same page for everyone. Data needs a session. |
| `/setup` | Passkey registration with a setup token in the URL fragment (6.6) |

Public API:

| Route | Result |
|---|---|
| `GET /api/posts` | Public posts, list fields |
| `GET /api/posts/{slug}` | One public post with `body_html`. Else 404. |
| `GET /api/topics` | Topics in order: `slug`, `name` |
| `GET /api/topics/{topic}` | Public posts of a topic |
| `GET /api/site` | Title section, with the intro as HTML |
| `GET /api/now` | Now box |
| `GET /api/surf` | Cached surf data |
| `GET /api/visitors` | Total |
| `GET /api/me` | `{ "owner": bool }`. Never 401. |
| `GET /feed.xml` | RSS |
| `GET /media/{key}` | Image (6.8) |
| `GET /healthz` | 200 if the DB and replication are fine |
| `GET /static/*` | CSS, JS, fonts, logo, WASM |

Public responses never change with the session.

Owner API (`/api/owner/*`). No valid session → 401. Every response has `Cache-Control: no-store`.

| Route | Result |
|---|---|
| `GET /api/owner/posts` | All posts, all states |
| `GET /api/owner/posts/{id}` | One post with `body_md`, `version` |
| `POST /api/owner/posts` | Create a draft |
| `PUT /api/owner/posts/{id}` | Save. Needs `If-Match: <version>`. Mismatch → 409. |
| `POST /api/owner/posts/{id}/state` | Set `state`. Needs `If-Match`. |
| `DELETE /api/owner/posts/{id}` | Delete |
| `PUT /api/owner/now` | Save the Now box |
| `GET /api/owner/site`, `PUT /api/owner/site` | Title section (4.9) |
| `POST /api/owner/topics` | Add a topic |
| `PUT /api/owner/topics` | Rename and order every topic |
| `DELETE /api/owner/topics/{topic}` | Delete a topic without posts |
| `POST /api/owner/uploads` | Upload an image (6.8) |
| `GET /api/owner/export.zip` | All posts, all states, as markdown |
| `GET /api/owner/passkeys` | List passkeys |
| `DELETE /api/owner/passkeys/{id}` | Revoke a passkey. Refused for the last one. |

Auth:

| Route | Result |
|---|---|
| `POST /auth/login/start`, `/auth/login/finish` | Passkey sign-in. Rate limited. |
| `POST /auth/register/start`, `/auth/register/finish` | Add a passkey. Needs a setup token or a session. |
| `POST /auth/logout` | End the session |

### 6.5 Database

Migrations are additive only. Never edit a shipped migration.

```sql
CREATE TABLE posts (
  id           INTEGER PRIMARY KEY,
  slug         TEXT NOT NULL UNIQUE,
  title        TEXT NOT NULL,
  summary      TEXT NOT NULL DEFAULT '',
  topic        TEXT NOT NULL REFERENCES topics (slug),   -- 0002 rebuilt the table without the fixed list
  tags         TEXT NOT NULL DEFAULT '[]',   -- JSON array
  body_md      TEXT NOT NULL,
  body_html    TEXT NOT NULL,
  word_count   INTEGER NOT NULL,
  state        TEXT NOT NULL DEFAULT 'draft' CHECK (state IN ('draft','private','public')),
  version      INTEGER NOT NULL DEFAULT 1,
  published_at TEXT,
  updated_at   TEXT NOT NULL
);

CREATE TABLE topics (slug TEXT PRIMARY KEY, name TEXT NOT NULL, position INTEGER NOT NULL);

CREATE TABLE site (                           -- one row (4.9)
  id INTEGER PRIMARY KEY CHECK (id = 1),
  title TEXT NOT NULL, subtitle TEXT NOT NULL, tagline TEXT NOT NULL,
  intro_md TEXT NOT NULL, updated_at TEXT
);

CREATE TABLE now_box (
  id INTEGER PRIMARY KEY CHECK (id = 1),
  body_md TEXT NOT NULL, body_html TEXT NOT NULL, updated_at TEXT NOT NULL
);

CREATE TABLE passkeys (
  id BLOB PRIMARY KEY, passkey TEXT NOT NULL, label TEXT NOT NULL,
  created_at TEXT NOT NULL, last_used_at TEXT
);

CREATE TABLE sessions (
  token_hash BLOB PRIMARY KEY,                -- SHA-256 of the cookie value
  created_at TEXT NOT NULL, expires_at TEXT NOT NULL
);

CREATE TABLE setup_tokens (
  token_hash BLOB PRIMARY KEY,
  expires_at TEXT NOT NULL, used_at TEXT
);

CREATE TABLE visits (day TEXT PRIMARY KEY, count INTEGER NOT NULL);

CREATE TABLE heartbeat (id INTEGER PRIMARY KEY CHECK (id = 1), at TEXT NOT NULL);
```

- SQLite settings: `journal_mode=WAL`, `busy_timeout=5000`. Litestream runs checkpoints. The app does not.
- The DB lives on a named Docker volume.
- `heartbeat` gets a write every 10 min, so the replica age always means something (6.12).
- No secret is stored in plain form. A leaked backup gives no way to sign in.

### 6.6 Sign-in

- One owner. No sign-up.
- **Setup token** (first passkey, or a lost passkey):
  - Only from the CLI: `docker compose exec logbook-app logbook setup-link`.
  - Prints `<origin>/setup#<token>` once, to stdout. Never written to logs.
  - The token is in the URL fragment, so the browser never sends it in a request line, a server log, or a `Referer` header. The page posts it in a JSON body.
  - Valid 15 min. Single use. Marked used in the same transaction that adds the passkey.
  - Only the hash is stored.
- More passkeys: from a session, in the editor.
- Revoke: `DELETE /api/owner/passkeys/{id}`. The last passkey cannot be revoked. Use the CLI for a full reset.
- RP ID = the host of `LOGBOOK_ORIGIN`. Subdomains are allowed, so passkeys also work on `drill.<DOMAIN>` (6.12).
- The RP ID cannot be an IP address. Local development uses `http://localhost:8080`.
- Session cookie: random 32 bytes. `HttpOnly`, `Secure`, `SameSite=Strict`, `Path=/`, 30 days. The DB stores only its SHA-256.
- CSRF, for every write:
  - `SameSite=Strict` cookie.
  - `Origin` header must equal `https://<DOMAIN>`.
  - `Content-Type: application/json` (uploads: `multipart/form-data` plus the `Origin` check).
  - No token needed.
- The blog domain must not be a subdomain of a sandcastle domain, so sandcastle is never "same-site".
- Rate limit on `/auth/*`: 20 requests per minute per client IP (`LOGBOOK_AUTH_RATE_LIMIT`). The client IP is the rightmost `X-Forwarded-For` entry that is not a trusted proxy. The app trusts that header only when the direct peer is in `LOGBOOK_TRUSTED_PROXIES` (the `edge` network).
- Passkey ceremonies (start → finish) live in memory for 5 minutes and work once.
- Route access levels in `routes.rs`: `Public` (never depends on the session), `Session` (`/api/me`: depends on the session, `no-store`, never 401), `Auth` (`/auth/*`: rate limited, `no-store`), `Owner` (401 without a session, `no-store`).

### 6.7 Markdown pipeline

`render(md) -> html`, one function, used by the server at save time and by the WASM preview:

1. comrak: CommonMark + GitHub tables, footnotes, task lists. Raw HTML is escaped and shows as text (comrak's default drops it silently).
2. syntect: code highlighting with classes, not inline styles (CSP).
3. ammonia: allow-list of tags and attributes. Links get `rel="noopener noreferrer"`. Only `http`, `https`, `mailto`, and relative URLs.

### 6.8 Uploads

- Max 10 MB.
- Allowed: PNG, JPEG, WebP, GIF. Checked by magic bytes, not by name or header.
- **No SVG.** Same-origin SVG is stored XSS.
- The server decodes and re-encodes each image. This removes EXIF (GPS) and other metadata. Limits: 8000 × 8000 pixels, 256 MB for decoding. An animated GIF keeps only its first frame.
- Key = `<sha256 hex>.<ext>`, made by the server. Stored in the bucket under `uploads/`.
- `/media/{key}`: key checked by `media_key_ok` (7.6). Fixed `Content-Type` from the extension. `X-Content-Type-Options: nosniff`. Long cache.
- Local disk cache (bucket only): max 1 GB; above that, the oldest files go until it is under 90 %.
- Without `S3_BUCKET`, images go to a local folder (`LOGBOOK_MEDIA_DIR`), for development and tests.
- The editor inserts `![](/media/<key>)` from the IMG button or a pasted image. The exports carry the images: the zip has all of them, the git export only the ones that public posts use.

### 6.9 HTTP headers

Set by the app on every response:

```
Content-Security-Policy: default-src 'self'; script-src 'self' 'wasm-unsafe-eval';
  style-src 'self'; img-src 'self' data:; font-src 'self'; connect-src 'self';
  object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'
Strict-Transport-Security: max-age=63072000; includeSubDomains
X-Content-Type-Options: nosniff
Referrer-Policy: strict-origin-when-cross-origin
Permissions-Policy: camera=(), microphone=(), geolocation=()
```

### 6.10 Export [decided; git export off in production]

- Each public post → `posts/<slug>.md` with a front-matter header: `title`, `slug`, `summary`, `topic`, `tags`, `state`, `published`, `updated`. Values are JSON strings, so any title round-trips, and other YAML readers parse them too. The Now box → `now.md`. Images → `images/` (step 6b).
- Target: a **separate public repo**, `logbook-posts`. Not the code repo. Reason: a deploy key can push any branch of its repo. On the code repo, a pushed workflow file could read the CI secrets, which include the root key of the shared VPS.
- The server pushes with a deploy key for `logbook-posts` only.
- Runs 10 s after each owner change (save, state, delete, Now box), and once a day. It commits only when a file changed. It needs `git` and `openssh-client` in the image.
- Only `public` posts. A post that leaves `public` disappears from the next export. Its old versions stay in git history.
- One-way. Edits in `logbook-posts` do not come back.
- Production does not use the git export (decided 2026-10-04). Public posts are in the same bucket backup as all other posts (6.12), so a second copy is not necessary. The code stays. It is off without `EXPORT_REPO`.
- Owner zip (`/api/owner/export.zip`): all posts, all states. Downloaded by the browser.

All posts exist only in the DB, its backup in the private bucket, and owner zips.

### 6.11 Hosting

Same Hetzner VPS as sandcastle. Same deploy patterns (`sandcastle/deploy/README.md`).

| | sandcastle | blog |
|---|---|---|
| Compose project | `/srv/sandcastle` | `/srv/logbook` |
| Data | Postgres | SQLite + Litestream |
| Repo | own | own, public |
| CI | own | own |

**Shared Caddy.** Sandcastle's Caddy owns ports 80 and 443.

- Docker network `edge` connects sandcastle's Caddy and the blog app.
- Docker network `edge` has the fixed subnet `172.30.0.0/24`. The app trusts `X-Forwarded-For` from that subnet only.
- Sandcastle's Caddyfile ends with `import /etc/caddy-sites/*.caddy`. Host folder `/srv/edge/sites/` is mounted at `/etc/caddy-sites/`. (Sandcastle mounts `/etc/caddy` read-only, so the folder cannot go inside it.) An empty folder is valid.
- The blog owns its file, `deploy/logbook.caddy`. The deploy replaces `__DOMAIN__` with `DOMAIN` from `.env`:

  ```
  __DOMAIN__ {
  	encode zstd gzip
  	reverse_proxy logbook-app:8080
  }
  ```

- Blog deploy, Caddy step:
  1. Fail with a clear message if sandcastle's Caddy is not running or not on `edge`. (So sandcastle deploys first on a new server.)
  2. Keep the old file as `.bak`. Write the new file. Run `caddy validate` on the full config inside the Caddy container.
  3. If valid: run `caddy reload`.
  4. If not valid: put the old file back and stop. Caddy did not reload, so the live config does not change.
  5. `curl https://<DOMAIN>/healthz` until it answers (2 min; the first deploy waits for the certificate).

  [verified 2026-10-04] Local test with `caddy:2`: import of the folder, validate, reload, and the rollback of a bad file.

One-time edits to sandcastle:

1. `compose.yaml`: add the external `edge` network to `caddy` (keep the default network). Mount `/srv/edge/sites:/etc/caddy-sites:ro`.
2. `Caddyfile`: add `import /etc/caddy-sites/*.caddy`.
3. Deploy script, before `docker compose up`: `docker network inspect edge >/dev/null 2>&1 || docker network create --subnet 172.30.0.0/24 edge` and `mkdir -p /srv/edge/sites`.

**Blog container:**

- `deploy/compose.yaml`: service `app`, alias `logbook-app` on `edge`. No host port.
- Memory limit 512 MB. Docker log rotation (10 MB × 3).
- Litestream inside the image. Start sequence: 6.12.

**Bucket:** `logbook`, private, in its **own Hetzner project** (not the sandcastle project). [verified 2026-10-04] Hetzner docs: "each key pair is automatically valid for every Bucket within the same project." Bucket policies can restrict keys, but a separate project is simpler.

| Prefix | Content | Writer |
|---|---|---|
| `db/` | SQLite replica | Litestream |
| `uploads/` | Images | App |

The server disk holds only rebuildable things: image, local DB copy, image cache.

**Hetzner S3 notes** (from sandcastle `tools/atlas/packs-sync.py`):

- Endpoint: `https://<location>.your-objectstorage.com`.
- Virtual-host addressing. Hetzner denies path-style writes. Litestream: `force-path-style: false`, set explicitly.
- Litestream: `sign-payload: true`, set explicitly. Test one real write to the bucket before the first deploy.
- Signed payloads. Hetzner denies unsigned uploads above about 20 MB.
- Region: `auto`.
- aws-sdk-rust: set request checksum calculation and response checksum validation to `WhenRequired`. The newer default (`WhenSupported`) breaks many S3-compatible stores. [build-check] One test upload to Hetzner.

**Secrets.** Same split as sandcastle. GitHub secrets hold the env file text. CI writes them with mode 600. All deploy secrets live in a GitHub **Environment** `production` that only `master` can use.

`STACK_ENV`:

```
IMAGE=ghcr.io/eserilev/blog
DOMAIN=unclebill.blog
```

`PROD_ENV`:

```
LOGBOOK_ORIGIN=https://unclebill.blog   # the passkey RP ID; never change it
S3_ENDPOINT=https://<location>.your-objectstorage.com
S3_BUCKET=logbook
S3_ACCESS_KEY=...
S3_SECRET_KEY=...
NWS_USER_AGENT=logbook (<contact email>)
S3_FORCE_PATH_STYLE=false   # Hetzner: virtual-host style; the bucket goes into the endpoint host
HEALTHCHECK_URL=...
# RESTORE_ONLY=1             # drills only (6.12)
```

`ALLOW_EMPTY_START` is a Deploy input, not a secret, so it cannot stay on by mistake. Examples: `deploy/.env.example`, `deploy/prod.env.example`.

Reused from sandcastle: `VPS_HOST`, `VPS_USER`, `VPS_SSH_KEY`, `VPS_FINGERPRINT`.

Keep `PROD_ENV` in a password manager. `PROD_ENV` + the bucket = the whole blog.

Public-repo CI rules: never use `pull_request_target`. Workflows from forks get no secrets.

### 6.12 Backups and recovery

**Container start:**

1. `litestream restore -if-db-not-exists -if-replica-exists`.
2. If there is still no DB:
   - `ALLOW_EMPTY_START=1` → create a new DB. (First deploy only. Remove the line after.)
   - Else → exit with "no replica found in <bucket>/db; refusing to start empty". A wrong bucket name never starts an empty site.
3. If `RESTORE_ONLY=1` → run the app with no replication. Local writes are thrown away. (Drills.)
4. Else → `litestream replicate -exec logbook`. Litestream exits when the app exits. Docker restarts the container.

[verified 2026-10-04] Litestream v0.5 docs: `restore -if-db-not-exists`, `restore -if-replica-exists`, `restore -timestamp`, and `replicate -exec` exist with this behavior. Pin one v0.5.x version in the Dockerfile.

**Litestream config** (`deploy/litestream.yml`, checked against v0.5.17): a `replica:` block per database with `type: s3`, `bucket`, `path: db`, `endpoint`, `region`, `force-path-style`, `sign-payload: true`. Top level: `snapshot.interval: 24h`, `snapshot.retention: 720h` (30 days; the default is 24h).

Point-in-time restore (`-timestamp`): exact only inside `l0-retention` (default 5 min). Older restore points land on compaction levels (hourly at L3) and daily snapshots, back 30 days.

**Bucket safety** [verified 2026-10-04]: Hetzner supports versioning and object lock. Lifecycle rules support noncurrent-version expiry (`NoncurrentDays`) only.

- Turn on versioning and object lock (governance mode, 35 days) on `logbook`. The app key can delete `db/`, but a deleted object stays as an old version. A compromised server cannot destroy the backups.
- Lifecycle rule: expire noncurrent versions after 40 days. Litestream deletes old files every day, so without this rule, storage grows forever.
- Object lock must be chosen when the bucket is created.

**Recovery** (the VPS is dead; both apps are down):

1. New VPS, as in sandcastle "One-time: the VPS".
2. Move the Primary IP to it, or update DNS A records. [verified 2026-10-04] Hetzner: a Primary IP moves only to a server in the same location, the target server must be powered off, and the IP must first be removed from the old server. So create the new VPS in the same location, power it off, move the IP, power it on.
3. Update `VPS_HOST` and `VPS_FINGERPRINT` in both repos.
4. Restore and deploy sandcastle (its runbook).
5. Run the blog workflow **Deploy** (`workflow_dispatch`, input: commit SHA, or empty for the head of `master`). It builds the image from that commit (cached) and runs no tests. It works at any time.
6. Open the site. Sign in with a passkey. Lost passkey → CLI setup link (6.6).

The blog needs no file from the old server.

If the old VPS is only unreachable and still running, power it off first. Two writers on one replica damage the backup.

**First deploy:** run Deploy by hand with `allow_empty_start` on. Then `docker compose -f /srv/logbook/compose.yaml exec app logbook setup-link` and register a passkey. Later deploys leave the flag off.

**Rollback:** run Deploy by hand with the old commit SHA. It tags that image `prod`. After a migration, an old image refuses the newer schema (sqlx). Then rollback needs a point-in-time restore to before the migration. Write this in the PR of every migration.

**Checks:**

- **Replica age** (only with a bucket): every 10 min, the first time 1 min after start, the app reads the age of the newest object under `db/`. Older than 1 h, or no replica, or an unreadable bucket → `/healthz` fails with 503.
- **Nightly backup check** (`checks.rs`, only with a bucket): every 24 h, the first time 1 h after start. `litestream restore -o <temp>`, then `PRAGMA integrity_check`, then the restored `heartbeat` must be less than 1 h old. Then ping `HEALTHCHECK_URL` (success) or `HEALTHCHECK_URL/fail`. A missed ping → email (healthchecks.io). The same check by hand: `logbook check-backup`.
- **Uptime:** an external monitor calls `/healthz`.
- **Drill, twice a year:** a test VPS with `RESTORE_ONLY=1`, at `drill.<DOMAIN>`. Check posts, images, and passkey sign-in (RP ID is the parent domain). `RESTORE_ONLY` means the drill never writes to the production replica.

### 6.13 Deploys

Workflow **CI** (push to `master`, PRs): all gates of 7.9.

Workflow **Deploy** (`.github/workflows/deploy.yml`):

- On each push to `master` (a merged PR): automatic, at once. It does not wait for CI on `master`, because the PR passed CI before the merge. Never for a PR or a fork. Only when the repo variable `DEPLOY_ENABLED` is `true`.
- By hand (`workflow_dispatch`, inputs: SHA, `allow_empty_start`): first deploy, recovery, and rollback.
- Steps: build `ghcr.io/eserilev/blog:sha-<commit>` → tag it `prod` → copy `compose.yaml` and `logbook.caddy` to `/srv/logbook` → write `.env`, `prod.env` (mode 600) → make sure that `edge` exists with its subnet and that sandcastle's Caddy is on it → `docker compose pull && docker compose up -d` → Caddy step and health check (6.11).
- One deploy at a time. A deploy in progress is never cancelled.
- Runs in the `production` Environment. Actions pinned to commits. Runbook: `deploy/README.md`.

### 6.14 Layout

```
blog/
├── Cargo.toml                 workspace
├── crates/
│   ├── logbook-core/          pure logic. No I/O, no async, no unsafe. Verified (7.6).
│   │   └── src/ policy.rs slug.rs media.rs post.rs
│   ├── logbook-render/        render(): comrak + syntect + ammonia. Server and WASM.
│   └── logbook-server/
│       ├── migrations/
│       └── src/ main.rs routes.rs config.rs db.rs auth.rs pages.rs posts.rs
│                now.rs surf.rs media.rs uploads.rs headers.rs checks.rs
│                feed.rs export.rs counter.rs cli.rs
├── editor-wasm/               logbook-render for the browser; build.sh → static/wasm/
├── static/                    index.html, CSS, JS components, fonts, logo
├── fuzz/                      cargo-fuzz targets, seed inputs in fuzz/seeds/
├── proofs/                    Lean 4: Aeneas output + proofs
├── e2e/                       Playwright
├── deploy/                    Dockerfile, entrypoint.sh, restore-test.sh, compose.yaml, compose.test.yaml,
│                              logbook.caddy, litestream.yml, *.env.example, README.md
├── .github/workflows/         ci.yml, deploy.yml, nightly.yml
└── designs/                   mockups
```

### 6.15 Build order

Each step ends working, with its tests.

1. **Scaffold.** Workspace, three crates, axum serving the split mockup (no inline scripts, self-hosted fonts, CSP). CI: fmt, clippy, deny, tests.
2. **Posts.** SQLite, `posts`, `render()`, `logbook-core` (`reveal`, slugs), public API, `<head>` tags, path router. Litestream start sequence with a local S3 (SeaweedFS). First fuzz targets. Restore test.
3. **Sign-in.** Passkeys, sessions, CSRF checks, CLI setup link, revoke. Route table + access matrix. Playwright passkey test.
4. **Proofs.** One-day Aeneas spike on `make_slug` and `filter_public` first. If the spike fails, cut the scope (7.6). Then the theorems and the CI job. (Done: T1–T5, T10–T12. T6–T9 follow in a later PR.)
5. **Editor.** Owner API, versions and 409, `<md-editor>`, WASM preview.
6. **Extras.** In two PRs. 6a: Now box, RSS, counter, export, route-kind test. 6b: uploads, surf report.
7. **Deploy.** Dockerfile, compose, Caddy step, workflows, sandcastle edits, Hetzner project and bucket, domain.
8. **Drill.** Full drill (6.12).

## 7. Testing and verification

| Layer | Tool | Catches |
|---|---|---|
| Unit | nextest | Wrong logic |
| Property | proptest | Unexpected edge cases |
| Fuzz | cargo-fuzz | Crashes, unsafe output on hostile input |
| Integration | in-process axum + SQLite | Wrong API behavior |
| Access matrix | test built from the route table | Leaks, missing session checks |
| Browser | Playwright | Broken components and sign-in |
| Restore | Compose + SeaweedFS (local S3) | A backup that cannot come back |
| Mutation | cargo-mutants | Tests that check nothing |
| Proofs | Charon + Aeneas + Lean 4 | Any input that breaks the `logbook-core` functions. Only those functions. |

### 7.1 Rules

- `#![forbid(unsafe_code)]` in every crate, except the `abi` module of `editor-wasm`: the WASM memory shim (three functions, each with a `SAFETY` comment). The rest of that crate denies unsafe code.
- `cargo fmt --check`, `cargo clippy -- -D warnings` with `clippy::pedantic`.
- `cargo deny`: licenses, duplicates, advisories.
- Each bug fix adds a test that fails without it.
- Coverage (`cargo llvm-cov`) reported, not a gate.

### 7.2 Property tests

- **Sanitizer:** for any markdown, the output has no `<script>`, `<iframe>`, `<object>`, `<style>`, `<svg>`, no `on*` attribute, no `javascript:` or `data:` URL. Checked by parsing with html5ever and walking every node.
- **Preview = server:** for any markdown, WASM `render()` and native `render()` give the same bytes.
- **Export round trip:** `parse(write(post)) == post`.
- **Head tags:** for any title and summary, the output `<head>` parses with exactly the expected elements.
- **Server = core:** for any post set, the public API returns exactly `filter_public` of it.

### 7.3 Fuzz targets

| Target | Input | Oracle |
|---|---|---|
| `render` | markdown bytes | No panic. Passes the sanitizer check. |
| `frontmatter` | export file bytes | No panic. Parses → round trip holds. |
| `post_input` | JSON body of `PUT /api/owner/posts/{id}` | No panic. Accepted input always meets the limits (title, summary, tags, body). |
| `slug` | title bytes + id | Passes the slug rules (4.3). |
| `media_key` | path bytes | No panic. Agrees with T10. |
| `head_tags` | title + summary + site title, plus a private post | Output never breaks out of an attribute. Never contains the private title. |
| `image` | image bytes | No panic in decode and re-encode. Output has no EXIF. |

Seed inputs live in `fuzz/seeds/<target>/`. The generated corpus is not committed. Each crash → a regression test. PR: 60 s per target. Nightly: 30 min per target, as a CI matrix.

### 7.4 Integration and access matrix

Access matrix (one test):

- Built from `routes.rs`, so every route is covered. A route with no expected result fails the test.
- Viewers: guest, owner, guest with a revoked session, guest with an expired session.
- Data: posts in all three states, with unique marker strings in each title and body.
- Checks per route: status code. No marker of a non-public post anywhere in a non-owner response (body and headers), page routes included. `Cache-Control: no-store` on every non-public route. Public routes give the same response to every viewer.
- The owner gets a fresh session for every request, because `/auth/logout` ends a session.

Other integration tests:

- Owner writes without a session → 401. Wrong `Origin` or wrong content type → 403.
- `If-Match` mismatch → 409.
- Rate limit on `/auth/*`, with the client IP from trusted `X-Forwarded-For` only.
- RSS and export: public posts only.
- Setup token: expires after 15 min, works once, never appears in logs.
- Last passkey cannot be revoked.
- Uploads: SVG refused, wrong magic bytes refused, EXIF removed, size limit.
- `/posts/{private}` and `/posts/{unknown}`: status 404, `noindex`, no marker.
- Security headers present on every route.
- Start with an empty bucket and no `ALLOW_EMPTY_START` → refuses to start.

### 7.5 Browser tests

Playwright, Chromium:

- Guest view: no owner controls.
- Passkey sign-in with the Chromium virtual authenticator: setup link → register → sign out → sign in.
- Editor: preview updates. Publish public, private, draft. Sign out → private and draft are gone.
- Two tabs edit one post → the second save shows the conflict message.
- Now box edit.
- No CSP violations in the console on any page.
- Keyboard: every control reachable, focus visible.
- Screenshot comparison: home, post, editor.

### 7.6 Formal verification (Aeneas)

Charon reads the Rust code. Aeneas translates it into a pure Lean 4 model. Each function returns `ok v`, or `fail` (panic, overflow, out of bounds). Theorems about the model are theorems about the Rust code.

Scope: `logbook-core` only. Its rules: no I/O, no async, no `dyn`, no `String` (bytes only), plain structs, enums, loops, `Option`, `Result`, `Vec`.

**Verified API:**

```rust
pub enum State { Draft, Private, Public }

pub struct Post {
    pub id: u64, pub state: State, pub topic: Topic, pub word_count: u32,
    pub slug: Vec<u8>, pub title: Vec<u8>, pub summary: Vec<u8>, pub tags: Vec<u8>,
    pub body_html: Vec<u8>, pub published_at: Option<Vec<u8>>, pub updated_at: Vec<u8>,
}

/// Safe for a guest. Private field. Only `reveal` builds it.
/// No Default, Deserialize, Clone-from-Post, or From<Post>.
pub struct PublicPost(Post);

pub fn reveal(p: Post) -> Option<PublicPost>;
pub fn filter_public(ps: &[Post]) -> Vec<PublicPost>;
pub fn make_slug(title: &[u8], id: u64) -> Vec<u8>;
pub fn media_key_ok(key: &[u8]) -> bool;
pub fn reading_minutes(words: u32) -> u32;
```

Guest code paths use `PublicPost` only. Owner paths use `Post`. A guest route that tries to send a `Post` does not compile.

**Theorems** (Lean names in `proofs/Logbook/`):

| # | Name | Statement | Meaning |
|---|---|---|---|
| T1 | `reveal_spec` | `reveal p = ok r → (r.isSome ↔ p.state = Public)` | Exactly the public posts pass. |
| T2 | `reveal_id` | `reveal p = ok (some q) → q.inner = p` | `reveal` does not change the post. |
| T3 | `filter_public_spec` | `filter_public ps = ok qs → qs.map inner = ps.filter (·.state = Public)` | The guest list is exactly the public posts, in order. |
| T4 | `slug_charset` | `make_slug t i = ok s → ∀ c ∈ s, c ∈ [a-z0-9-]` | No `.`, `/`, `\` ever. |
| T5 | `slug_shape` | `make_slug t i = ok s → 1 ≤ s.len ≤ 80 ∧ s[0] ≠ '-' ∧ s[s.len-1] ≠ '-' ∧ "--" ∉ s` | Never empty, short, clean. |
| T6 | `slug_plain` | `t ∈ [a-z0-9]⁺ ∧ t.len ≤ 80 → make_slug t i = ok t` | A plain title is its own slug. Rules out a function that ignores its input. |
| T7 | `slug_fallback` | `(∀ c ∈ t, c ∉ [A-Za-z0-9]) → make_slug t i = ok ("post-" ++ dec i)` | A title with no ASCII letters or digits gets `post-<id>`. |
| T8 | `slug_lower` | `make_slug t i = ok s → make_slug (upper t) i = ok s` | Case does not matter. |
| T9 | `slug_idempotent` | `make_slug t i = ok s → make_slug s i = ok s` | Stretch goal. Cut if it costs more than a week. |
| T10 | `media_key_spec` | `media_key_ok k = ok b → (b ↔ k ∈ [0-9a-f]{64} "." ("png" ∣ "jpg" ∣ "webp" ∣ "gif"))` | Accepts exactly the keys that the server makes. Nothing that can leave the folder. |
| T11 | `reading_spec` | `reading_minutes w = ok m → m = max 1 ⌈w / 220⌉` | Exact formula. |
| T12 | `total_*` | For each function `f` and every input `x`: `∃ v, f x = ok v` | No panic, overflow, or out-of-bounds access, for any input. |

T12 turns every "if `ok`" above into "always".

**Status (2026-10-04):**

| Theorem | Status | Lean |
|---|---|---|
| T1, T2 | Proved | `reveal_spec`, `reveal_iff` |
| T3 | Proved | `filter_public_spec` |
| T4, T5 | Proved | `make_slug_spec` (`ValidSlug`), `slug_charset` |
| T10 | Proved | `media_key_spec` (`keyShape`) |
| T11 | Proved | `reading_spec` |
| T12 | Proved for all five functions | the `⦃ ⦄` form of each theorem |
| T6, T7, T8, T9 | Not yet proved. Property tests cover them (`slug.rs`). | — |

`Logbook/Axioms.lean` makes the build fail if any proved theorem depends on an axiom other than `propext`, `Classical.choice`, and `Quot.sound`.

**Not covered by proofs:** `render()` (property tests + fuzz), sessions, passkeys, CSRF (integration + Playwright), SQL and route code (access matrix), the tools themselves (rustc, Charon, Aeneas, Lean).

**What the spike showed.** Aeneas handles every function in `logbook-core`, including loops over slices of structs with `Vec<u8>` fields. The Rust code keeps away from std helpers that Aeneas does not model: `u8::is_ascii_*`, `contains` on ranges, `Vec::is_empty`, and `Option::clone` (so `Post` has a hand-written `Clone`). The derived `Debug` and `PartialEq` of `Option` stay as generated axioms, because no verified function calls them.

**Setup:**

- `proofs/tools.sh` downloads pinned Charon and Aeneas release builds and checks their SHA-256. No Nix.
- The Charon version must match the `charon-pin` of the Aeneas build. Aeneas refuses files from another Charon version.
- Charon drives the Rust nightly in its `rust-toolchain` file (with `rustc-dev`, `llvm-tools`, `rust-src`).
- `proofs/extract.sh`: Rust → Charon → Aeneas → `proofs/LogbookCore/{Types,Funs,FunsExternal}.lean`. These files are generated and committed.
- `proofs/Logbook/*.lean` hold the theorems. `lean-toolchain` follows the Aeneas build. Mathlib comes from `lake exe cache get`.
- CI: run `extract.sh`, fail on a diff in `proofs/LogbookCore`, then `lake build Logbook`, and fail on any `sorry`.

### 7.7 Restore test

Per PR, `deploy/restore-test.sh` with `deploy/compose.test.yaml` (app + SeaweedFS as a local S3, because MinIO no longer publishes images):

1. Start with `ALLOW_EMPTY_START=1`. Create posts in all states, an image, a passkey.
2. `logbook check-backup` must pass (the nightly check, 6.12).
3. Stop the app. Delete its volume.
4. Start a new container without the flag on the same bucket.
5. Check: all posts, the image, the passkey, the access rules.
6. Point at an empty bucket without the flag → the app refuses to start.

### 7.8 Mutation testing

cargo-mutants makes small changes (`==` → `!=`, deletes `!`, replaces a body with `true`) and runs the tests. A change that no test notices is "missed": the code is not really tested.

Example: deleting a check in `reveal` that makes drafts public. Missed → add the test "a guest cannot see a draft".

- Nightly: `logbook-core`, `auth.rs`, `posts.rs`, `pages.rs`, `media.rs`, `uploads.rs`, `export.rs`.
- PR: `cargo mutants --in-diff`.
- `.cargo/mutants.toml` excludes logging and metrics.
- A missed mutant in `logbook-core`, `auth.rs`, `posts.rs`, or `pages.rs` fails the job.

### 7.9 CI

PR and push to `master`. A PR must pass before the merge. A merge deploys at once (6.13). All jobs run in parallel, each on its own machine:

1. fmt, clippy, deny.
2. Unit + property.
3. Integration + access matrix.
4. Playwright.
5. Restore test. It builds the release image, about 150 s of the 240 s. Caching the dependency build (cargo-chef) is open work.
6. Fuzz, 60 s per target, one job per target.
7. Proofs. Only when `crates/logbook-core/`, `proofs/`, `rust-toolchain.toml`, or `ci.yml` changes. Otherwise the job is skipped, and GitHub counts it as passed.
8. Mutants `--in-diff` (not built yet).

Measured 2026-10-05: about 6.5 min with proofs (proofs 392 s, restore 241 s, fuzz about 150 s). Without proofs, the restore test sets the time (about 4 min).

Nightly: fuzz matrix (30 min per target), full mutants, `cargo deny` with fresh advisories. A crash or a failure opens an issue.

## 8. Content rules

- Plain, serious copy. The design carries the personality.
- No invented personal facts.
- No 88×31 badges.
- No Netscape.

## 9. Rejected ideas

- Black and yellow construction stripe. Rainbow WordArt. Ticker. Blinking text.
- Colored topics, badges, toolbar icons.
- Title "Eitan" (too plain). Title "~eitan".
- "Netscape" text. Sidebar badges.
- Invented details (board size, belt, WoW character, schedule).
- Export to the code repo, on a branch (6.10).
- Private posts in a public export, encrypted (key leak risk, metadata leak).
- Open-Meteo for the surf report. NOAA covers the one spot, with real buoy data.

## 10. Open questions

1. GitHub, X, Discord handles.
2. Now box content.
3. Final name.
4. Logo: keep the "E" with waves?
5. RSS: keep?
6. ~~Webring.~~ Removed (2026-10-05).
7. Counter: real or fixed?
8. About page content.
9. ~~Domain.~~ `unclebill.blog` (2026-10-04).
10. Words per minute: 220?
