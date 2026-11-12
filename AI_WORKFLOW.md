# AI workflow note

<!--
Skeleton. The facts below are filled in from PROCESS.md; the judgement in each
"TODO" is yours to write. Delete this comment and the TODO markers before submitting.
-->

## Tools used

- Claude Code (Anthropic) in VS Code, as the main coding agent.
- Claude Code subagents and workflows: a Sonnet subagent for each section's frontend, and small review workflows (independent reviewers plus verifiers) after the backend of sections 1 and 2 and for sharing.
- TODO: anything else (search, docs, other assistants).

## How the work was set up

- A project instruction file (`CLAUDE.md`) fixed the stack, the build order (one section of the brief at a time), and the working rules: plan first and wait for approval, gate every step with a check, stop and report instead of guessing, commit per section.
- The AI wrote a time-boxed plan as a checklist; it was re-budgeted against the real deadline before any code was written.
- Backend code was written test-first at two agreed boundaries (the HTTP API and the import converter). Frontend work started only after that section's API was tested, with the shared types and routes as a fixed contract.
- A running log of decisions, failures and rejections was kept in [PROCESS.md](PROCESS.md).

TODO: why you chose this setup (one or two sentences).

## Where AI materially sped up the work

Facts to choose from (see PROCESS.md for detail):

- Scaffolding a Rust workspace with Axum, Yew, Trunk and a multi-stage Dockerfile that built and deployed on the first try.
- The Quill-to-WebAssembly glue and the autosave state machine.
- The Markdown-to-Delta converter with 27 hand-specified test cases.
- The test harness that drives the real router against a throwaway Postgres.
- Independent review passes that found real defects before a reviewer would (see below).

TODO: which of these mattered most to you, and roughly how much time they saved.

## What AI output was changed or rejected

Facts to choose from:

- The first plan did not fit the deadline and included nothing for the video; it was re-budgeted and features were cut up front (delete, unshare, save-on-unload).
- A password-less "user picker" login was proposed; email and password login was chosen instead.
- The AI first launched a review with up to 9 agents; it was stopped and capped at 5.
- An AI reviewer's finding about the Trunk binary was rejected by a second AI verifier with evidence (the same Dockerfile had already deployed).
- A reviewer's suggested fix for oversized documents (raise limits, support images) was rejected in favour of a one-line allowlist in the editor.
- Review findings on the import converter (hard line breaks dropped, code blocks inside list items mangled, byte-order mark hiding the first heading) were fixed test-first.
- TODO: anything you rewrote by hand or pushed back on in conversation.

## How correctness, UX and reliability were verified

- Automated: server tests through the real HTTP router and a real Postgres (sign-in, access rules, sharing, import, validation); unit tests for the web client's pure logic. Counts are in SUBMISSION.md.
- Manual, by me, on the live deploy after each section: the gate checks listed in the plan (create, format, rename, refresh; import; share between two accounts; a third account gets "not found").
- The AI could not run the server, reach the live site or push to GitHub from its sandbox; every deploy and browser check was done by hand.
- TODO: what you looked at in the code yourself, and anything you would not trust without checking.

## What I would do differently

TODO.
