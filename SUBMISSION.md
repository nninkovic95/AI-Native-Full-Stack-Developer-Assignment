# Submission

## Links

- **Live product:** https://ai-native-full-stack-developer-assignment-p9to.onrender.com
  (free tier: the first request after 15 idle minutes takes about a minute)
- **Source:** https://github.com/nninkovic95/AI-Native-Full-Stack-Developer-Assignment
- **Walkthrough video:** see [VIDEO_URL.txt](VIDEO_URL.txt)

## Test accounts

| Email | Password |
| --- | --- |
| `alice@ajaia.test` | `ajaia-demo` |
| `bob@ajaia.test` | `ajaia-demo` |
| `carol@ajaia.test` | `ajaia-demo` |

Suggested review path (about three minutes):

1. Sign in as Alice. Create a document, type, apply bold, a heading and a list, rename it. Refresh: everything is still there.
2. On the document list, click "Import file" and choose `samples/sample.md` from this repository. It opens as a formatted document. Try a `.docx`: it is refused with a clear message.
3. Open a document, click "Share", enter `bob@ajaia.test`.
4. In a private window, sign in as Bob. The document is under "Shared with me", marked as shared by Alice. Bob can edit it; Bob cannot share it.
5. Sign in as Carol. The document is not listed, and its direct URL says it was not found.

## What is included

| Item | Where |
| --- | --- |
| Source code | this repository (`crates/`, `migrations/`, `scripts/`, `Dockerfile`) |
| Setup, run, test and deploy instructions | [README.md](README.md) |
| Architecture note | [ARCHITECTURE.md](ARCHITECTURE.md) |
| AI workflow note | [AI_WORKFLOW.md](AI_WORKFLOW.md) |
| Process log (decisions, failed attempts, rejected AI output) | [PROCESS.md](PROCESS.md) |
| Sample files for the import feature | [samples/](samples/) |
| Walkthrough video URL | [VIDEO_URL.txt](VIDEO_URL.txt) |
| The brief and the working rules given to the AI | [CLAUDE.md](CLAUDE.md) |

## What works

| Brief section | Status | Notes |
| --- | --- | --- |
| 1. Create, rename, edit, save, reopen | Working | Bold, italic, underline, two heading levels, bulleted and numbered lists; autosave with a visible status |
| 2. File upload | Working | `.txt` and `.md` (UTF-8, up to 1 MB) become new documents; supported types are stated in the UI and the README |
| 3. Sharing | Working | Owner shares by email; "My documents" and "Shared with me" are separate; shared users can edit but not share; others get "not found" |
| 4. Persistence | Working | Postgres; formatting, sessions and shares survive refreshes and redeploys |
| 5. Engineering quality | Working | Live deploy, validation with readable errors, automated tests, architecture note |

Automated tests: 107 in total. 82 on the server (55 send real HTTP requests through the router to a real Postgres: sign-in 5, documents 8, import upload 9, sharing 13, validation 20; 27 cover the import converter) and 25 unit tests for the web client's logic.

## What is incomplete or intentionally left out

- No real-time collaboration; simultaneous edits to one document overwrite each other (last save wins).
- Access cannot be revoked and documents cannot be deleted from the UI.
- One sharing level only (edit); no view-only or comment roles.
- No images in documents; no `.docx` import.
- No sign-up or password reset; accounts are seeded.
- No browser-driven tests; pages were checked by hand on the live deploy.
- Closing the tab within about a second of the last keystroke can lose that keystroke.
- None of the optional stretch items were attempted.

## With another 2 to 4 hours

1. **Revoke access and delete documents**, with tests for both (about 45 minutes). They complete the sharing story.
2. **A version check on save** so a stale tab cannot silently overwrite newer edits; show "this document changed elsewhere" instead (about an hour). It is the honest first step towards collaboration.
3. **View-only sharing**: a role on the share, enforced in the one access check, and a read-only editor (about an hour).
4. **Browser-driven smoke test** of the main flow against a local server, run in CI (about an hour).
5. Smaller items: save on tab close, local time in the document list, hashing session tokens at rest, bundling the editor library instead of loading it from a CDN.
