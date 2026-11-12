# AI workflow note

## Tools used

- Claude Code (Anthropic) in VS Code, as the main coding agent.
- Claude Code subagents and workflows:
  - a Sonnet subagent for each section's frontend;
  - a review workflow with independent reviewers and verifiers after the section 1 backend;
  - a single independent reviewer inside the file-import and sharing workflows;
  - a review workflow over the web client, and a fact-check of the docs against the code.

## How I used AI

- Backend test-first (TDD); frontend tests only where plain `cargo test` works.
- Contract first: the backend and the shared DTOs pass their tests before Sonnet subagents build the UI.
- Agent swarm capped at 5.
- Each extra (dark mode, delete, revoke, export) in its own commit so I could revert it.

How that was set up:

- A project instruction file (CLAUDE.md) fixed the stack, the build order (one section of the brief at a time), and the working rules: plan first and wait for approval, gate every step with a check, stop and report instead of guessing, commit per section.
- The AI wrote a time-boxed plan as a checklist; it was re-budgeted against the real deadline before any code was written.
- Backend tests sit at two agreed boundaries: the HTTP API and the import converter.
- A running log of decisions, failures and rejections was kept in PROCESS.md.

## Where AI materially sped up the work

- Scaffolding a Rust workspace with Axum, Yew, Trunk and a multi-stage Dockerfile that built and deployed on the first try.
- The Quill-to-WebAssembly glue and the autosave state machine.
- The import converter (plain text and Markdown to Delta) with 27 tests, 20 of them with a hand-written expected Delta.
- The test harness that drives the real router against a throwaway Postgres.
- Running work in parallel where files did not overlap: the sharing UI, the validation backend and an access-control review at the same time, while the docs were written in the main session.
- Independent review passes that found real defects before a reviewer would (see "How I verified").

## What I changed or rejected from the AI

- Stack: the AI suggested FastAPI. I chose Rust end to end (Axum + Yew + Quill) because I've built with Yew before.
- Plan fixes: I corrected the deadline, removed vault rules leaking into the repo, added the cookie flags (HttpOnly, SameSite, Secure), and turned Render auto-deploy off.
- I rejected a redundant test seam (an access-policy function), since the HTTP API tests already cover it.
- I rejected one reviewer false alarm after checking it myself.
- Region: US East for Render and Neon, with Neon's direct (non-pooled) connection string.

Also changed or rejected during the build (detail in PROCESS.md):

- The first plan did not fit the deadline and included nothing for the video; it was re-budgeted and features were cut up front (delete, unshare, save-on-unload). Delete, revoking access, Markdown export and a dark theme were added at the end, once the core was deployed, each by a small agent workflow in its own commit. Version history was considered and declined: it could not be built and checked safely in the time left.
- A password-less "user picker" login was proposed; email and password login was chosen instead.
- The AI first launched a review with up to 9 agents; it was stopped and capped at 5.
- A reviewer's suggested fix for oversized documents (raise limits, support images) was rejected in favour of a one-line allowlist in the editor.
- Review findings on the import converter (hard line breaks dropped, code blocks inside list items mangled, byte-order mark hiding the first heading) were fixed test-first.
- The reviewer's suggested fix for the double click on "New document" (component state only) was replaced by a synchronous flag.
- A fact-check of the AI-written docs against the code found statements that were wrong or misleading (where the save status is shown, how renames are saved, what happens to links and tables on import, review steps that could not be followed literally); the docs were corrected.
- The delete confirmation was first the browser's own confirm box; it was changed to an in-page dialog because a native dialog does not show in a browser-tab recording.

## How I verified

- Live gate check on the deployed URL after every section: login, edit, import, sharing alice -> bob, carol 404, revoke, delete, Download .md round trip.
- Review agents found real bugs: pasted images breaking the editor, Ctrl+Z after open wiping the doc, a double click creating two docs. All fixed and checked live.
- Automated: server tests through the real HTTP router and a real Postgres (sign-in, access rules, sharing, revoking access, delete, import, validation); unit tests for the web client's pure logic. Counts are in SUBMISSION.md.
- The AI could not run the server, reach the live site or push to GitHub from its sandbox; every deploy and browser check was done by hand.

## What I'd do next

- Polish the theming, both light and dark.
- Markdown on the fly: type Markdown in the editor and see it rendered live.
- Version history: saved snapshots per doc that you can restore.
- Real-time collaboration last, since it's the hardest part (presence plus merging concurrent edits).
