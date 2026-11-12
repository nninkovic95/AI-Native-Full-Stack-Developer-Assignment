# AI workflow note

<!--
Skeleton. The facts below are filled in from PROCESS.md; the judgement in each
"TODO" is yours to write. Delete this comment and the TODO markers before submitting.
-->

## Tools used

- Claude Code (Anthropic) in VS Code, as the main coding agent.
- Claude Code subagents and workflows:
  - a Sonnet subagent for each section's frontend;
  - a review workflow with independent reviewers and verifiers after the section 1 backend;
  - a single independent reviewer inside the file-import and sharing workflows;
  - a review workflow over the web client, and a fact-check of the docs against the code.
- TODO: anything else (search, docs, other assistants).

## How the work was set up

- A project instruction file ([CLAUDE.md](CLAUDE.md)) fixed the stack, the build order (one section of the brief at a time), and the working rules: plan first and wait for approval, gate every step with a check, stop and report instead of guessing, commit per section.
- The AI wrote a time-boxed plan as a checklist; it was re-budgeted against the real deadline before any code was written.
- Backend code was written test-first at two agreed boundaries (the HTTP API and the import converter). Frontend work started only after that section's API was tested, with the shared types and routes as a fixed contract.
- A running log of decisions, failures and rejections was kept in [PROCESS.md](PROCESS.md).

TODO: why you chose this setup (one or two sentences).

## Where AI materially sped up the work

Facts to choose from (see PROCESS.md for detail):

- Scaffolding a Rust workspace with Axum, Yew, Trunk and a multi-stage Dockerfile that built and deployed on the first try.
- The Quill-to-WebAssembly glue and the autosave state machine.
- The import converter (plain text and Markdown to Delta) with 27 tests, 20 of them with a hand-written expected Delta.
- The test harness that drives the real router against a throwaway Postgres.
- Running work in parallel where files did not overlap: the sharing UI, the validation backend and an access-control review at the same time, while the docs were written in the main session.
- Independent review passes that found real defects before a reviewer would (see below).

TODO: which of these mattered most to you, and roughly how much time they saved.

## What AI output was changed or rejected

Facts to choose from:

- The first plan did not fit the deadline and included nothing for the video; it was re-budgeted and features were cut up front (delete, unshare, save-on-unload). Delete and a dark theme were added at the end, once the core was deployed, each by a small agent workflow in its own commit.
- A password-less "user picker" login was proposed; email and password login was chosen instead.
- The AI first launched a review with up to 9 agents; it was stopped and capped at 5.
- An AI reviewer's finding about the Trunk binary was rejected by a second AI verifier with evidence (the same Dockerfile had already deployed).
- A reviewer's suggested fix for oversized documents (raise limits, support images) was rejected in favour of a one-line allowlist in the editor.
- Review findings on the import converter (hard line breaks dropped, code blocks inside list items mangled, byte-order mark hiding the first heading) were fixed test-first.
- The web client review found that Ctrl+Z after opening a document emptied it and the empty document was then autosaved, and that a double click on "New document" created two documents. Both were fixed. The reviewer's suggested fix for the double click (component state only) was replaced by a synchronous flag.
- A fact-check of the AI-written docs against the code found statements that were wrong or misleading (where the save status is shown, how renames are saved, what happens to links and tables on import, review steps that could not be followed literally); the docs were corrected.
- TODO: anything you rewrote by hand or pushed back on in conversation.

## How correctness, UX and reliability were verified

- Automated: server tests through the real HTTP router and a real Postgres (sign-in, access rules, sharing, import, validation); unit tests for the web client's pure logic. Counts are in [SUBMISSION.md](SUBMISSION.md).
- Manual, by me, on the live deploy: create, format, rename, refresh; TODO: list the import, sharing ("Document not found" for a third account), undo and dark mode checks you actually ran.
- The AI could not run the server, reach the live site or push to GitHub from its sandbox; every deploy and browser check was done by hand.
- TODO: what you looked at in the code yourself, and anything you would not trust without checking.

## What I would do differently

TODO.
