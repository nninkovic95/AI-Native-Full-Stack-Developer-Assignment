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
