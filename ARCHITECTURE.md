# Architecture note

## What was prioritised, and why

The brief asks for a coherent product slice under a tight timebox, with access logic and persistence that actually work. The order of work followed that:

1. **A deployed skeleton first.** The first commit that did anything was a hello-world page served by the API, built by Docker and live on Render. Later sections were deployed the same way and checked by hand on the live URL, so deployment was never a last-hour risk.
2. **The editing loop.** Create, format, rename, autosave, reopen. This is the part a reviewer touches first, so it had to feel finished: a visible save status, inline rename, formatting that survives a reload.
3. **Access rules on the server.** Who may open, edit and share a document is decided on the server and tested through the HTTP API. The UI only reflects it.
4. **A file import that is useful rather than broad.** Two formats done properly, with clear rejections for everything else.

Depth went into the access rules, the import conversion and the tests. Breadth (roles, real-time editing, export, more file types) was cut on purpose.

## Shape of the system

```
Browser                       Render (one Docker container)                Neon
┌──────────────────────┐      ┌───────────────────────────────────┐       ┌──────────┐
│ Yew app (WebAssembly) │ ───► │ Axum                              │ ────► │ Postgres │
│ Quill editor          │ JSON │  /api/*   sessions, documents,    │ SQLx  └──────────┘
└──────────────────────┘      │           shares, import          │
                              │  /*       the built frontend      │
                              └───────────────────────────────────┘
```

One process serves both the API and the static frontend, so there is one thing to deploy and no cross-origin setup. The session cookie is `HttpOnly` and `SameSite=Lax`, and `Secure` when `COOKIE_SECURE=true`, as it is on the live deploy.

A `shared` crate holds the request and response types. The server and the web client both compile against it, so a change to one of those types that one side does not follow is a compile error rather than a runtime surprise. Route paths and status codes are not covered by this; the HTTP tests cover them on the server side.

## Decisions

**Document content is stored as a Quill Delta (JSON), not HTML.** A Delta is the editor's own lossless format, so what is saved is exactly what is shown again. It is also data rather than markup: the server checks its shape, and there is no stored HTML to sanitise before showing it to another user.

**One access rule.** A single query, `load_accessible`, answers "may this user open this document": they own it, or there is a share row for them. Every route that takes a document ID goes through it. The document list applies the same owner-or-share condition in its own query. Sharing, revoking access and deleting add one more rule on top: only the owner may do them.

**No access looks like not found.** A user without access gets 404, the same answer as for an ID that does not exist, so document IDs cannot be probed. A shared user who tries to share gets 403, because they already know the document exists.

**Markdown import is converted on the server, to the same Delta format.** Imported and typed documents are then the same kind of thing everywhere else in the system. The converter is a pure function from file name and bytes to a title and a Delta, which made it cheap to test thoroughly: 27 tests, 20 of which state the expected Delta by hand, the rest covering titles and rejected files. Markdown that has no equivalent in the editor (links, code, quotes) keeps its words and loses its formatting rather than being dropped; a link keeps its text but not its address. Tables are not parsed: their raw syntax comes through as one plain paragraph.

**Sessions in the database.** Signing in creates a random token stored in a `sessions` table with an expiry; signing out deletes it. Passwords are hashed with Argon2. Accounts are seeded because sign-up is not what the brief is testing.

**Autosave instead of a Save button.** Edits to the text are saved about a second after typing stops, and from the first edit on the page shows "Unsaved changes", "Saving…", "Saved" or the error. Edits made while a save is in flight trigger one more save when it returns. The title is saved when its field loses focus or Enter is pressed.

**Every API error has one shape.** Any non-2xx answer from `/api` is `{"error": "<readable message>"}`, including malformed requests, oversized bodies and unknown paths, so the web client can always show the server's message.

**Tests at two boundaries.** Server tests send real HTTP requests through the real router to a real Postgres, one fresh database per test; they assert only on HTTP responses. The import converter is tested as a pure function. In the web client, logic that does not need a browser (the autosave state machine, validation, request shapes, sharing rules, theme choice) lives in plain modules with ordinary unit tests. There are no browser-driven tests; pages were checked by hand on the live deploy.

**Queries are checked at run time, not compile time.** SQLx can verify SQL against a live database during compilation, but then the Docker build needs a database. The HTTP tests exercise every query instead.

## Scope cuts

| Cut | Why | What it would take |
| --- | --- | --- |
| Real-time collaboration, conflict handling | The largest feature in the space; a half-built version is worse than an honest "last save wins" | A CRDT or OT layer and WebSockets; a version check on save would be the cheap first step |
| View-only and comment roles | The brief asks for a simple sharing model; one clear rule beats three half-tested ones | A `role` column on the share row, checked in the one access query |
| Version history | Needs a new table, a snapshot policy and a restore path through the editor; too much to add safely at the end | Snapshots on save at most every few minutes, a history panel, restore |
| `.docx` import | Needs a real parser to be done properly | A conversion library on the server, feeding the same Delta builder |
| Links and tables on import | The editor's toolbar offers neither, so they would be content a user cannot edit | Enable the link format in the editor; convert tables to plain rows |
| Images in documents | They would be stored inline and grow documents past sensible request sizes | Object storage and an upload route |
| Sign-up, password reset | Seeded accounts are enough to show sharing | Standard account flows |
| Browser-driven tests | Slow to set up for a WebAssembly frontend within the timebox | Playwright against a local server |
