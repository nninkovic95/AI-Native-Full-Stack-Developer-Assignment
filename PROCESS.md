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
- First deploy worked on the first try: the Docker build took 2m50s on Render's free builder, well under the 5–15 min the plan feared. The Dockerfile was never built locally.
- Neon is used through its direct (non-pooled) connection string, which avoids PgBouncer's limits on prepared statements for SQLx and the boot-time migrations.
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
- The AI sandbox cannot run the server or reach the live URL, so the browser gate is run by hand. Passed on the live deploy: signed in as alice, created a document, applied bold, a heading and a list, renamed it, refreshed; title and formatting persisted. First time the Quill interop and the Secure cookie ran for real, and both worked unchanged.
- A review workflow over the backend and the never-built Dockerfile was first launched with up to 9 agents, then stopped and capped at 5 (three reviewers, verifiers only for the most severe findings).
- Review result (5 agents: three reviewers, two verifiers):
  - Confirmed and fixed: Quill accepted pasted or dropped images as base64 even though the toolbar has no image button; a document over axum's 2 MB default body limit would then fail every autosave. Fix chosen: a `formats` allowlist in the Quill glue (one line), which also drops `<img>` from pasted HTML. Rejected the larger fix (raising the body limit and supporting images): images are out of scope.
  - Rejected by the verifier: "the unpinned Trunk binary may not run on bookworm". The same Dockerfile had already built and deployed.
  - Not applicable: a warning about SQLx's migration lock behind Neon's pooled URL; the deploy uses the direct connection.
  - Deferred to the validation pass (section 5): unknown `/api/*` paths fall through to the SPA and answer 200 with HTML, and axum's own rejections (malformed JSON, oversized body, bad UUID) are plain text instead of the `{error}` JSON shape.
  - The auth, session and access-control review found no defect.

## Section 2: File import

- The converter started in the main session (plain text, headings), then the rest of the section moved to a workflow capped at 5 agents: a backend agent working test-first; once its tests pass, a Sonnet UI agent and an independent read-only reviewer in parallel; a repair agent only if the reviewer finds a real defect or the UI does not build. The workflow stops after the backend step if its tests are not green, so no UI is built against an unfinished contract.
- Result: 4 of 5 agents used, about 35 minutes of wall-clock against 15 planned. The backend agent took 8 minutes; the UI agent 15 and the reviewer 20. Slower than working directly, but the reviewer found two real converter defects that the author's tests had missed:
  - a code block or quote inside a list item fused with the item's text and left an empty bullet;
  - a Markdown hard line break was dropped, joining the words on either side.
  Both were reproduced with a failing test first, then fixed by the repair agent. Its choice, kept: a hard break inside a list item or heading becomes a space, because Quill has no line break inside a line and splitting would create a second list item.
- Fixed in the main session, test-first: a UTF-8 byte-order mark hid the first heading of a `.md` file (reviewer finding, low severity).
- Integrator check before committing: full server suite (49 tests), web unit tests (18) and `trunk build` re-run in the main session; read the upload handler (session checked before the body is read, explicit body limit on the route, size limit answered as JSON 413).
- Workflow script bug, harmless: the "UI not clean" condition treated the UI agent's free-text "no contract problems" note as a problem, which gave the repair agent a non-issue to reject. Fixed in the next workflow by using a boolean field.
- Both agents reported using shell heredocs to write code against a standing rule, and one skipped the red step for two slices that the previous slice already satisfied; both were disclosed in their reports. The next workflow's brief forbids heredocs explicitly.
- Changed plan for speed: section 5's validation backend runs in parallel with section 3's UI (different crates), and the docs are written in the main session while agents work.
- Markdown is converted to a Quill Delta on the server rather than to HTML: the stored format stays the same for typed and imported documents, and the conversion is a pure function that can be tested against hand-written expected Deltas.

## Section 3: Sharing, with section 5's validation in parallel

- One workflow, 4 of 5 agents used, about 12 minutes (the section 2 workflow took 35): a sharing backend agent first (test-first, 13 HTTP tests); once green, three agents at once on files that do not overlap: the sharing UI (Sonnet, `crates/web`), the validation backend (`crates/server`), and a read-only access-control reviewer. Briefs carried explicit time targets and narrower reading lists this time.
- The contract (routes, status codes, exact error messages) was written into the workflow before any agent started, so the UI agent and the reviewer worked from the same text as the backend agent.
- Access-control review: no defect found. It confirmed that every route that takes a document ID goes through the one access check (the list route repeats the same owner-or-share condition in its own query), that the owner check comes after it (strangers get 404, shared users 403), and that a duplicate share is decided by the primary key rather than a pre-check that could race.
- Sharing decisions: a shared user can see who else has access but cannot share onward; an unknown email is a 400 with a plain message (the demo accounts are public, so there is nothing to hide); sharing twice is a 409.
- TDD honesty: 4 of the 13 sharing tests passed on first run because `load_accessible` already honoured share rows from section 1. They stay as the proof that sharing works end to end.
- Validation: titles are trimmed and limited to 200 characters, content must be a Quill Delta, and every non-2xx API answer now has the `{error}` JSON shape, including malformed JSON, non-UUID ids, oversized bodies, unknown API paths and wrong methods (20 tests). This closes the two items deferred from the section 1 review.
- Integrator check before committing: full server suite (82 tests), web unit tests (25) and `trunk build` re-run in the main session; read the share handlers and the editor page change (the share panel is rendered outside the editor wrapper so opening it does not re-create Quill; to be confirmed in the browser gate).
- A second, read-only workflow (cap 4) reviews the committed web client in parallel, since the UI had no independent review so far.
- Web client review result (4 agents, about 6 minutes, reading the committed code while the UI agent edited the working tree):
  - Confirmed and fixed (high): pressing Ctrl+Z after opening a document emptied it, and the empty document was then autosaved. Quill's history records the programmatic load even with the "silent" source; the comment in the glue file had assumed otherwise. Fix: clear the undo history after loading. The verifier read Quill's source to confirm both the defect and the fix.
  - Confirmed and fixed (medium): "New document" could be clicked twice while the request was pending, creating a second empty document that cannot be deleted. Fix: a pending flag checked synchronously, plus a disabled "Creating…" state. The reviewer's suggested fix relied on component state alone, which a second click in the same frame can still pass; changed to a synchronous flag.
  - Not fixed, low severity, listed as known limitations: a failed autosave is retried only on the next edit; sign-in redirects add history entries, so Back from the sign-in page returns to it.
  - Both fixes were applied in the main session rather than by a subagent: two small edits with the code already verified.
- Dark mode (requested once the core was deployed; not one of the brief's stretch items): one Sonnet agent, time-boxed, in its own commit so it can be dropped. Colours moved to CSS variables, a dark palette that follows the system setting, a toggle that remembers the choice, and overrides for Quill's hard-coded light toolbar colours. Pure logic (stored choice versus system preference) has 5 unit tests. Integrator check: tests and `trunk build` re-run, read the toggle component and the inline script; `Cargo.lock` unchanged (only web-sys feature names were added). The look itself can only be judged in a browser.
- Docs fact-check (2 read-only agents, 174 claims traced to the code): the product was fine, the AI-written docs were not. Corrected: the save status is at the top right, not next to the title; a rename is saved on Enter or blur, not by autosave; imported links lose their address and tables are not converted; "every test uses the router and a database" was true for 55 of 82; the review steps could not be followed literally (the file picker hides `.docx`; Carol has to sign in before pasting the URL); "one access check" overstated it, since the list query repeats the condition; claims that every section was checked live were reworded to what is logged. Lesson kept in the AI note: generated docs need the same independent check as generated code.
- Delete document, requested after the core was deployed (it was cut from the first plan): a two-agent workflow, backend test-first, then the list button by a Sonnet agent; about 4 minutes. Owner only (shared users get 403, others 404), shares removed by the existing cascade, confirmation dialog in the UI. The first backend slice was honestly red in a useful way: the minimal delete let a stranger delete the document, which the next test caught before the access check was added. Integrator check: 88 server tests, 33 web tests and `trunk build` re-run; read the handler.
- Live checks by hand, confirmed working on the deployed site: file import, sharing between accounts, and dark mode. These were the first browser runs of the share panel and the theme toggle; neither needed a change.
- Two more extras chosen after that, built in parallel on files that do not overlap, each in its own commit: revoking access (backend test-first, then the Remove button) and "Download .md" (a tested Delta-to-Markdown converter in the web client, one of the brief's stretch items). Feature freeze set for 07:45 to leave time for a full live check before recording.
- Result of the two extras: 3 agents, about 4 minutes. Revoking access: 7 HTTP tests, owner only, a shared user cannot remove anyone including themself. Markdown export: 17 unit tests for the converter; underline is dropped because Markdown has no form for it. The UI agent for revoke added one thing beyond its brief, kept: sharing and removing wait for each other, so a removal cannot overwrite the list with a stale copy.
- Version history was asked for with about 25 minutes of build time left. Declined after estimating it honestly: a new table on the live database, a snapshot policy, three endpoints and a restore path through the editor, with four other new features still unchecked in a browser. Listed as the first next step instead.
- Visual polish: one Sonnet agent, CSS only (one file, no markup changes), so it could run next to the feature agents and cannot change behaviour. It worked without seeing a browser; the look is judged by hand on the live site, and the change is its own commit so it can be reverted alone.
- The delete confirmation was first the browser's own confirm box. Changed to an in-page dialog (one Sonnet agent, under a minute): a native dialog does not appear in a browser-tab recording and ignores the theme. A product reason found only by preparing the demo.
- Final integrator run before the last deploy: 95 server tests, 52 web tests, frontend build. The release build could not finish in the AI sandbox because it downloads an optimiser; Render runs that step.
- The docs (README, ARCHITECTURE, SUBMISSION, AI_WORKFLOW skeleton) were written in the main session while the agents worked, then checked against the built behaviour before being committed.
