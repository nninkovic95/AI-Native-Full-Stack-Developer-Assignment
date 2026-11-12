# Ajaia Docs

A lightweight collaborative document editor: create and format documents in the browser, import text or Markdown files, and share documents with other users.

**Live:** https://ai-native-full-stack-developer-assignment-p9to.onrender.com

The live app runs on Render's free tier, which sleeps after 15 minutes without traffic. The first request after a sleep takes about a minute.

## Demo accounts

| Email | Password |
| --- | --- |
| `alice@ajaia.test` | `ajaia-demo` |
| `bob@ajaia.test` | `ajaia-demo` |
| `carol@ajaia.test` | `ajaia-demo` |

To see sharing: sign in as Alice, create a document, share it with `bob@ajaia.test`, then sign in as Bob (a private window keeps both sessions open). Carol is the user with no access.

## What it does

- **Documents:** create, rename, edit and reopen. Formatting: bold, italic, underline, two heading levels, bulleted and numbered lists.
  - The text saves automatically about a second after you stop typing. A status at the top right of the page shows "Unsaved changes", "Saving…", "Saved" or "Save failed: …".
  - The title saves when you press Enter or click outside it.
- **File import:** "Import file" on the document list turns a file into a new, editable document.
  - Supported types: **`.txt` and `.md` only**, UTF-8, up to **1 MB**.
  - Markdown headings, bold, italic, bulleted and numbered lists (including nested ones) keep their formatting.
  - Other Markdown is reduced to its text: a link keeps its text but loses its address, code and quotes become plain lines, and a table comes through as one line of raw text.
- **Sharing:** the owner of a document grants access to another user by email. The document list separates "My documents" from "Shared with me" and shows each document's owner. A user a document is shared with can open, edit and rename it; only the owner can share it. Anyone else gets "Document not found".
- **Persistence:** documents, formatting, sessions and shares are stored in Postgres.
- **Light and dark themes:** the app follows the system setting; the button in the bottom-right corner switches theme and remembers the choice.

## Stack

| Part | Choice |
| --- | --- |
| Frontend | Rust, [Yew](https://yew.rs) compiled to WebAssembly, built with [Trunk](https://trunkrs.dev) |
| Editor | [Quill 2](https://quilljs.com), through a small `wasm-bindgen` glue module |
| Backend | Rust, [Axum](https://github.com/tokio-rs/axum); it also serves the built frontend |
| Database | Postgres through [SQLx](https://github.com/launchbadge/sqlx) (Neon in production) |
| Deploy | One Docker image on Render |

```
crates/shared   request and response types used by both the server and the web client
crates/server   Axum API, sessions, access checks, file import, migrations runner
crates/web      Yew pages, Quill glue, pure logic modules with unit tests
migrations      SQL schema
samples         files to try the import with
scripts         test helper
```

## Run it locally

You need Rust (stable), the WebAssembly target, Trunk and a Postgres database.

```sh
rustup target add wasm32-unknown-unknown
cargo install --locked trunk

# A database; any Postgres 14+ works.
docker run -d --name ajaia-pg -e POSTGRES_USER=ajaia -e POSTGRES_PASSWORD=ajaia \
  -e POSTGRES_DB=ajaia -p 5432:5432 postgres:17

# Build the frontend into ./dist, then start the server.
(cd crates/web && trunk build)
DATABASE_URL=postgres://ajaia:ajaia@localhost:5432/ajaia cargo run -p server
```

Open http://localhost:3000 and sign in with a demo account. The server creates the tables and the demo accounts on start.

| Variable | Default | Purpose |
| --- | --- | --- |
| `DATABASE_URL` | required | Postgres connection string |
| `PORT` | `3000` | Port to listen on |
| `DIST_DIR` | `dist` | Folder with the built frontend |
| `COOKIE_SECURE` | unset | Set to `true` behind HTTPS so the session cookie is marked `Secure` |

For frontend work with live reload, keep the server running with the command above, run `trunk serve` in `crates/web` and open http://localhost:8080. Trunk forwards `/api` to port 3000.

## Tests

```sh
# Server: 55 tests send real HTTP requests through the router to a real Postgres;
# 27 more test the import converter directly.
scripts/with-test-db.sh cargo test -p server

# Web client: pure logic (autosave state machine, validation, request shapes,
# sharing rules, theme choice).
cargo test -p web
```

`scripts/with-test-db.sh` starts a throwaway Postgres for the run and stops it afterwards. It needs the Postgres server programs (`initdb`, `pg_ctl`) on the `PATH` but no running service. Without them, point the tests at any Postgres whose user may create databases: `DATABASE_URL=postgres://ajaia:ajaia@localhost:5432/ajaia cargo test -p server`. Each database test gets its own fresh database.

What the server tests cover: sign-in and the session cookie's flags, who can open and edit a document (owner, shared user, stranger), sharing rules, the document list, file import (conversion and rejections) and input validation.

## Deploy

The `Dockerfile` builds the server and the frontend and produces one small image.

1. Create a Postgres database (the live app uses Neon's free tier with the direct, non-pooled connection string).
2. On Render, create a Web Service from this repository with the Docker runtime on the free instance type.
3. Set `DATABASE_URL` and `COOKIE_SECURE=true`. Render provides `PORT`.

The server applies migrations and creates the demo accounts when it starts, so there is no separate setup step.

## Known limitations

- **No real-time collaboration.** Two people editing the same document at once overwrite each other; the last save wins.
- **Autosave window:** closing or reloading the tab within about a second of the last keystroke can lose that keystroke. Navigating inside the app saves first.
- **Renaming:** a new title is saved on Enter or when the title field loses focus; reloading while still typing in it loses the new title.
- **Failed saves** are shown in the status at the top right and retried on the next edit, not on a timer. Leaving the page while the status says "Save failed" loses the unsaved edits.
- **Browser Back on the sign-in page** returns to the sign-in page when signed out.
- **Images are not supported.** Pasted or dropped images are ignored.
- **Sharing is all-or-nothing:** a shared user can edit; there are no view-only or comment roles, and access cannot be revoked in the UI.
- **Documents cannot be deleted** in the UI.
- **Accounts are seeded.** There is no sign-up or password reset.
- **Import** covers `.txt` and `.md`; `.docx` is not supported. Imported links lose their address and Markdown tables are not converted.
- The editor library loads from a CDN, so the editor needs internet access.
- Times in the document list are shown in UTC.

See [ARCHITECTURE.md](ARCHITECTURE.md) for what was prioritised and why, and [SUBMISSION.md](SUBMISSION.md) for the list of deliverables.
