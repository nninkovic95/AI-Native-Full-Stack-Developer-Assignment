# Process log

A running log of key decisions, failed attempts, and changes or rejections during the build. Newest entries at the bottom of each section.

## Planning

- Read the brief and fixed the build order to the brief's sections (scaffold + deploy first, then editing, upload, sharing, persistence, quality).
- AI-proposed plan needed ~3h40m; re-budgeted to fit a hard deadline with 25m reserved for the video and Drive upload. Cut up front: delete document, unsharing, flush-on-unload, local Docker build gate.
- Login: chose email + password (argon2-hashed seeded users, DB-backed session cookie) over a password-less user picker; it costs ~15m but behaves like a real product.
- Cookie flags set for Render's HTTPS: `HttpOnly; SameSite=Lax; Path=/`, `Secure` in production via `COOKIE_SECURE=true`. Login must be verified on the live URL, not just locally.
- Render auto-deploy is off; deploys are triggered manually at each section's deploy step.
- Content stored as Quill Delta JSON (`JSONB`), not HTML: lossless for every format the editor offers, and the server can validate its shape.
- File upload: `.txt` and `.md` imported as a new document, converted to Delta on the server. `.docx` rejected as too costly for the timebox.
- No-access responses are 404, not 403, so users cannot probe which document IDs exist.
- SQLx runtime queries instead of compile-time macros, so the Docker build needs no database.

- Testing: TDD (red → green, one slice per cycle) on the backend only, at two agreed seams: the HTTP API (real router + real Postgres per test) and the file-import converter (pure function). The frontend is checked by gate checks, not browser tests.
- Frontend split: each section's Yew UI is delegated to a Sonnet subagent, but only after that section's backend and shared DTOs are tested. The DTOs and routes are the fixed contract. Pure web-side logic (state, request building, validation, debounce) lives in plain modules and is test-driven natively. The main session reviews the diff, builds, runs the gate and commits.

## Section 0: Scaffold + deploy

- Cargo workspace: `shared` (API types for both sides), `server` (Axum), `web` (Yew). `default-members` excludes `web` so root `cargo build`/`cargo test` stays native.
- Axum serves the Trunk `dist/` with an `index.html` fallback so client-side routes survive a refresh.
- Dockerfile installs Trunk from its prebuilt release binary rather than `cargo install`, to keep Render's build time down.
- The AI sandbox cannot reach crates.io, GitHub (push) or the Docker socket: dependencies for every section were declared up front so one `cargo fetch` by the user covered them; pushes are done by the user.

## Section 1: Editing (backend)

- Tests could not reach a Postgres on the host (the sandbox has its own network namespace and blocks Unix sockets). Rejected: running tests outside the sandbox by hand (too slow per red/green cycle). Chosen: install the Postgres server binaries and start a throwaway instance per test run (`scripts/with-test-db.sh`), TCP only, data under `$TMPDIR`. This also gives reviewers a one-command test run.
- Session token: two random UUIDv4s concatenated, stored in a `sessions` table with a 7-day expiry; no signing key to manage. Cookie `Max-Age` needed the `time` crate, already in the tree through `cookie`.
- Login returns the same 401 message for an unknown email and a wrong password, so accounts cannot be enumerated.
- One `load_accessible` query is the single access check for read and write; owner or a share row, otherwise 404.
- `PATCH` uses `COALESCE` so a title-only or content-only update leaves the other field alone; tests cover both.
- Argon2 is built with `opt-level = 3` in the dev profile; unoptimised hashing made every test and local start slow.
- Session token: note for reviewers that it is stored unhashed; acceptable for demo accounts, listed as a next step.
- TDD note: the wrong-password test passed on the first run because the login implementation already covered it; it stays as a regression test.

## Section 1: Editing (frontend, by subagent)

- Delegated to a Sonnet subagent after the backend tests were green, with the shared DTOs and routes as the fixed contract. It returned login, list and editor pages, a 27-line JS glue file for Quill, and 12 native unit tests for the pure logic (autosave state machine, form validation, request shapes).
- Integrator review: re-ran `cargo test -p web` and `trunk build` (both pass), read the editor page, the JS glue and the fetch wrappers. Accepted without changes.
- Accepted limits, to be stated in the README: a hard refresh inside the 800 ms autosave window can drop the last keystrokes; "last edited" is shown in UTC; Quill comes from a CDN.
- The subagent skipped the red step for one slice (`failure_from_response`) and said so; the test was kept.
- Not yet verified in a browser: the AI sandbox cannot run the server, so the create/format/rename/refresh gate is run by hand.
- A review workflow over the backend and the never-built Dockerfile was first launched with up to 9 agents, then stopped and capped at 5 (three reviewers, verifiers only for the most severe findings).
