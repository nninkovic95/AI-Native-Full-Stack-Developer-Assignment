# Submission

## Links

- **Live product:** https://ai-native-full-stack-developer-assignment-p9to.onrender.com
  (free tier: the first request after 15 idle minutes takes about a minute)
- **Source:** https://github.com/nninkovic95/AI-Native-Full-Stack-Developer-Assignment
- **Walkthrough video:** see VIDEO_URL.txt

## Test accounts

| Email | Password |
| --- | --- |
| `alice@ajaia.test` | `ajaia-demo` |
| `bob@ajaia.test` | `ajaia-demo` |
| `carol@ajaia.test` | `ajaia-demo` |

Suggested review path (about four minutes):

1. Sign in as Alice. Create a document, type, apply bold, a heading and a list. Rename it and press Enter. Wait for "Saved" at the top right, then refresh: everything is still there.
2. On the document list, click "Import file" and choose `samples/sample.md` from this repository. It opens as a formatted document. To see a rejection, switch the file picker's filter to "All files" and choose a `.docx` or `.pdf`: it is refused with "Only .txt and .md files can be imported".
3. Open a document, click "Share", enter `bob@ajaia.test`. Copy the document's URL.
4. In a private window, sign in as Bob. The document is under "Shared with me", marked "Shared by Alice". Bob can edit it; Bob's Share panel says only the owner can share.
5. Sign out and sign in as Carol. The document is not listed. Paste the URL copied in step 3: it says "Document not found".
6. Back as Alice: in the Share panel, click "Remove" next to Bob; the document disappears from Bob's list. Click "Download .md" to export the document. On the document list, "Delete" asks in a dialog before removing a document. The sun and moon button in the bottom-right corner switches between light and dark.

## What is included

| Item | Where |
| --- | --- |
| Source code | this repository (`crates/`, `migrations/`, `scripts/`, `Dockerfile`) |
| Setup, run, test and deploy instructions | README.md |
| Architecture note | ARCHITECTURE.md |
| AI workflow note | AI_WORKFLOW.md |
| Process log (decisions, failed attempts, rejected AI output) | PROCESS.md |
| Sample files for the import feature | samples/ |
| Walkthrough video URL | VIDEO_URL.txt |
| The brief and the working rules given to the AI | CLAUDE.md |

## What works

| Brief section | Status | Notes |
| --- | --- | --- |
| 1. Create, rename, edit, save, reopen | Working | Bold, italic, underline, two heading levels, bulleted and numbered lists; autosave with a visible status; owners can delete their documents |
| 2. File upload | Working | `.txt` and `.md` (UTF-8, up to 1 MB) become new documents; supported types are stated in the UI and the README |
| 3. Sharing | Working | Owner shares by email; "My documents" and "Shared with me" are separate; shared users can edit but not share; others get "Document not found" |
| 4. Persistence | Working | Postgres; formatting, sessions and shares survive refreshes and redeploys |
| 5. Engineering quality | Working | Live deploy, validation with readable errors, automated tests, architecture note |

Also shipped and checked on the live site:

- **Export to Markdown** ("Download .md" in the editor), one of the brief's optional stretch items.
- **Deleting documents** (owner only, confirmed in an in-page dialog).
- **Revoking access** ("Remove" in the Share panel, owner only).
- **Dark mode** with a sun and moon toggle; the app follows the system setting by default.
- **A sticky editor toolbar** that stays in view while scrolling a long document.

Automated tests: 147 in total. 95 on the server (68 send real HTTP requests through the router to a real Postgres: sign-in 5, documents 8, delete 6, import upload 9, sharing 13, revoking access 7, validation 20; 27 cover the import converter) and 52 unit tests for the web client's logic, 17 of them for the Markdown export.

## What is incomplete or intentionally left out

- No real-time collaboration; simultaneous edits to one document overwrite each other (last save wins).
- One sharing level only (edit); no view-only or comment roles.
- No images in documents; no `.docx` import; imported links lose their address and Markdown tables are not converted.
- No sign-up or password reset; accounts are seeded.
- No browser-driven tests; pages were checked by hand on the live deploy.
- Closing the tab within about a second of the last keystroke can lose that keystroke; a failed save is retried only on the next edit.
- Of the brief's optional stretch items, only export to Markdown was built; no version history, comments or collaboration indicators.

## With another 2 to 4 hours

1. **Polish the theming**, both light and dark.
2. **Markdown on the fly:** type Markdown in the editor and see it rendered live.
3. **Version history:** saved snapshots per document that you can restore.
4. **Real-time collaboration** last, since it is the hardest part: presence plus merging concurrent edits.
