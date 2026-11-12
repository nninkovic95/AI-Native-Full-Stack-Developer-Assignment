# Architecture note

## What was prioritised, and why

The brief asks for a coherent product slice under a tight timebox, with access logic and persistence that actually work. The order of work followed that:

1. **A deployed skeleton first.** The first commit that did anything was a hello-world page served by the API, built by Docker and live on Render. Every later section was verified on the live URL, so deployment was never a last-hour risk.
2. **The editing loop.** Create, format, rename, autosave, reopen. This is the part a reviewer touches first, so it had to feel finished: a visible save status, inline rename, formatting that survives a reload.
3. **Access rules on the server.** Who may open, edit and share a document is decided in one place on the server and tested through the HTTP API. The UI only reflects it.
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

One process serves both the API and the static frontend, so there is one thing to deploy and no cross-origin setup. The session cookie is `HttpOnly`, `SameSite=Lax` and `Secure` in production.

A `shared` crate holds the request and response types. The server and the web client both compile against it, so a change to the API that one side does not follow is a compile error rather than a runtime surprise.

## Decisions

**Document content is stored as a Quill Delta (JSON), not HTML.** A Delta is the editor's own lossless format, so what is saved is exactly what is shown again. It is also data rather than markup: the server can check its shape, and there is no stored HTML to sanitise before showing it to another user.

**One access check.** A single query answers "may this user open this document": they own it, or there is a share row for them. Every document route goes through it. Sharing adds one more rule on top: only the owner may share.

**No access looks like not found.** A user without access gets 404, the same answer as for an ID that does not exist, so document IDs cannot be probed. A shared user who tries to share gets 403, because they already know the document exists.

**Markdown import is converted on the server, to the same Delta format.** Imported and typed documents are then the same kind of thing everywhere else in the system. The converter is a pure function from file name and bytes to a title and a Delta, which made it cheap to test thoroughly: each test states the expected Delta by hand. Markdown that has no equivalent in the editor (links, code, quotes, tables) keeps its text and loses its formatting, rather than being dropped.

**Sessions in the database.** Signing in creates a random token stored in a `sessions` table with an expiry; signing out deletes it. Passwords are hashed with Argon2. Accounts are seeded because sign-up is not what the brief is testing.

**Autosave instead of a Save button.** Edits are saved about a second after typing stops, and the page always says whether the document is saved. Edits made while a save is in flight trigger one more save when it returns.

**Tests at two boundaries.** Server tests send real HTTP requests through the real router to a real Postgres, one fresh database per test; they assert only on HTTP responses. The import converter is tested as a pure function. In the web client, logic that does not need a browser (the autosave state machine, validation, request shapes) lives in plain modules with ordinary unit tests. There are no browser-driven tests; pages were checked by hand on the live deploy after each section.

**Queries are checked at run time, not compile time.** SQLx can verify SQL against a live database during compilation, but then the Docker build needs a database. The HTTP tests exercise every query instead.

## Scope cuts

| Cut | Why | What it would take |
| --- | --- | --- |
| Real-time collaboration, conflict handling | The largest feature in the space; a half-built version is worse than an honest "last save wins" | A CRDT or OT layer and WebSockets; a version check on save would be the cheap first step |
| View-only and comment roles, revoking access | The brief asks for a simple sharing model; one clear rule beats three half-tested ones | A `role` column on the share row and a delete route |
| Deleting documents | Not on the core path for a review | One route, one confirmation dialog |
| `.docx` import | Needs a real parser to be done properly | A conversion library on the server, feeding the same Delta builder |
| Images in documents | They would be stored inline and grow documents past sensible request sizes | Object storage and an upload route |
| Sign-up, password reset | Seeded accounts are enough to show sharing | Standard account flows |
| Browser-driven tests | Slow to set up for a WebAssembly frontend within the timebox | Playwright against a local server |
