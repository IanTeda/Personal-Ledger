# Documents

Documents is your library of proof: receipts, statements, insurance policies, warranties, tax papers and identity scans. A file only matters for what it proves, so each one links to the records it backs up, such as the Transaction for a fridge, the Payee that issued a policy or the Bill it settles. Personal Ledger keeps the links and details; the files themselves stay on disk beside your ledger and are never renamed or moved.

> **Heads up:** The Documents screen is built in the desktop app only, and it runs on sample data for now, so nothing you add or file is saved. Reading text out of scanned files (OCR), watched folders, dropping files onto the window and reminders that actually notify you aren't built yet.

## What you can do

- Browse every filed document, narrowed by type or financial year
- Search by file name or by the text read from inside a document
- Sort by newest or by soonest key date
- See what each document is linked to, and open the linked record
- Open a file, or show it in its folder
- Add a file by typing its path, or import several into the Inbox at once
- Triage the Inbox: accept a suggested link, file by hand, or skip
- Accept every strong match in one go, and undo it
- Fix what was read from a file before you file it
- Spot documents whose renewal, end or expiry date is close

## Terminology

- **Document:** A reference to one file kept beside your ledger, plus its details: a title, a type, a date, an optional key date and any number of links.
- **Document type:** What kind of document it is. Exactly one of Receipt, Statement, Tax, Insurance, Warranty & manual, Contract, Identity or Bill.
- **Filed / Unfiled:** A filed document is in the Library. An unfiled one is waiting in the Inbox. Only you file a document; adding a link doesn't.
- **Link:** A document's reference to one record it proves: a Transaction, an Inventory item, an Account, a Payee or a Bill plan.
- **Financial year:** The twelve months a document's date falls in, starting 1 July, such as FY 2025–26. It is worked out from the document date.
- **Key date:** The one forward-looking date a document may carry: Renews, Ends, Expires or Revalue by, with an optional reminder.
- **Need review:** A filed document whose key date is within 60 days either side of today.
- **Inbox:** The list of unfiled documents.
- **Suggested link:** The Transaction Personal Ledger thinks an unfiled document belongs to. It is recalculated each time and never saved.
- **Signals:** Which of amount, date and payee agree with the suggestion. All three is a strong match.

## Documents concept

Documents has two modes on one screen. You move between them with `i`, or by picking Inbox or All documents in the index rail.

### Library

Every filed document. The index rail on the left offers All documents, a Type list and a Financial year list, each with a count. The middle is a searchable list, and the right shows the selected document: its details, what it is linked to, and any date flag.

### Inbox

Files that haven't been filed yet. Each row shows where the file came from and a suggested link with a match meter. The right shows what was read from the file, the suggested link and a couple of other candidates. You accept, skip or file it by hand.

## Approach

1. Press `g` `f` to open Documents.
2. Bring files in with `I` (Import…), which puts them in the Inbox. Or in the Library press `a` to add one that is already filed.
3. In the Inbox, check what was read from the file. Press `e` if it's wrong.
4. Press `y` to accept the suggestion, `l` to pick a different record, or `x` to skip for now.
5. Back in the Library, press `l` to change a document's links and `shift`+`l` to jump to a linked record.
6. Sort by Expiring (`s`) to see what is coming due.

## Worked example

You buy a fridge for $1,899.00 and scan the receipt. You press `g` `f`, then `I`, and type the scan's path. The file lands in the Inbox as "fridge-receipt". The Inbox reads $1,899.00 and a merchant name, and suggests your Harvey Norman Transaction from the same week. The meter shows amount, date and payee all match, so it's a strong match. You press `y`: the receipt is filed as a Receipt under FY 2026–27 and a message tells you it was filed, with `u` to undo. Later you press `a` in the Library to add the three-year warranty, give it an Ends key date and tick the reminder. It shows up under Warranties & manuals with an "Ends" flag, and it counts toward "need review" once the date is within 60 days.

## Screens

### Library

The header shows how many files are filed. Below it are the search box, `Import…` and `+ Add`, then a line such as "12 of 31 · 3 need review". Each row has the title, type, date, link chips (a Transaction's chip shows its amount, an unlinked document says "not linked") and a date flag such as "Renews 12 Nov" or "Expired 3 Aug". Date flags within 60 days are red.

The detail pane shows a placeholder for the first page, the file type, pages and size, then Type, Document date, Financial year and any key date, followed by LINKED TO and the Open and Show in folder buttons.

### Inbox

Two columns: File, with the source (Scanned, Emailed, Downloads, Watched folder, Dropped or Imported) and date, and Suggested link, with a match meter and the signals that agree. A file with no readable amount says "Unreadable", and one with nothing close says "No suggestion". `Accept all strong matches · N` sits at the top, and `Watched folder…` says it isn't available yet.

### Dialogs

- **Add document / Edit document:** file path (Add only), title, document type, document date, key date kind and date, and a reminder tick.
- **Import files:** one path per line. Press `ctrl`+`enter` to import.
- **Edit extracted facts:** merchant, date, total and type. Clearing the total marks the file unreadable.
- **Link picker:** transactions near the document's date first, then accounts, payees, bill plans and inventory items. Type to narrow, `tab` to filter by kind.
- **Accept all strong matches:** says how many documents it will file before you confirm.

## Rules to know

- A document has exactly one type and any number of links, including none.
- Filing is your decision. Adding a link to a filed document doesn't move it, and a filed document with no links is fine.
- Only a Transaction can be suggested, and only if its amount or payee agrees with the file and it's within 7 days of the date. The date counts as agreeing within 3 days.
- A file with no readable amount never gets a suggestion. File it by hand or skip it.
- Skip lasts for the session: skipped files drop to the bottom of the Inbox.
- Undo reverses only the last accept, pick or accept-all.
- A document can't be sent back to the Inbox. Fix a misfiled one in place with `e` and `l`.
- Open and Show in folder use your system. If the file isn't at its path, you get a message saying where it should be.
- Files are never renamed or moved.

## Tips

- **Accept all is safe for strong matches:** it only files documents where amount, date and payee all agree, and `u` brings them back.
- **Search the money:** the link picker finds a Transaction by amount (`4380` or `4,380`) or by date (`14 sep`).
- **Use key dates for the dull stuff:** insurance renewals and warranty ends are the ones you'll thank yourself for.

## Scope

- Reading text out of files (OCR) and drawing real page previews aren't built. The text and previews you see are samples.
- Watched folders and dropping files onto the window aren't built.
- Raising a notification from a key date isn't built; the Notifications screen is its own piece of work.
- Attaching a file from a Transaction or Inventory item, with an `:attach` command, isn't built.
- Documents isn't in the terminal app.
- Nothing is saved: files, links and filings reset when you restart.

## Related

- **Transactions:** the records a receipt or statement most often proves, and what the Inbox matches against. See [Transactions](transactions.md).
- **Payees:** a document can link to a payee, and the link opens it in Settings. See [Payees](payees.md).
- **Accounts:** a statement can link to an account, and the link opens it in Settings. See [Accounts](accounts.md).
- **Bills:** a policy or contract can link to the bill plan that pays for it. See [Bills](bills.md).
- **Needs attention:** the ledger-wide list. Documents that need review don't join it. See [Needs attention](needs-attention.md).

## Getting around

| Go to | Terminal app | Desktop app |
| --- | --- | --- |
| Documents | not in the terminal app yet | `g` `f` (or `:documents`) |

Inside Documents, `i` flips between the Library and the Inbox, `h` and `l` move between the index rail and the list, and `:documents inbox`, `library`, `add`, `import` and `accept-all` run the same actions from the command box. The `?` screen lists the keys for the mode you're in.

| Key | Library | Inbox |
| --- | --- | --- |
| `j` `k` | Move between rows | Move between rows |
| `/` | Search | Not used |
| `enter` | Open the file | Open the file |
| `o` | Show in folder | Show in folder |
| `l` | Change links | Link elsewhere |
| `shift`+`l` | Follow a link | Follow the suggested link |
| `e` | Edit details | Edit extracted facts |
| `s` | Switch Newest and Expiring | Not used |
| `a` | Add a document | Not used |
| `y` | Not used | Accept |
| `shift`+`y` | Not used | Accept all strong matches |
| `x` | Not used | Skip |
| `u` | Not used | Undo |
| `shift`+`i` | Import files | Import files |

For the full set of keys, see [Getting around](getting-around.md).

## Feature set and requirements

Intended features for Documents. Ticked means built in at least one app; the tag says which.

- [x] DOC-001 (desktop): Browse filed documents by type and financial year, with counts
- [x] DOC-002 (desktop): Search file names and extracted text, and sort by newest or soonest key date
- [x] DOC-003 (desktop): Show a document's details and what it links to
- [x] DOC-004 (desktop): Link a document to a Transaction, Inventory item, Account, Payee or Bill plan, and follow the link
- [x] DOC-005 (desktop): Add a filed document and edit its details
- [x] DOC-006 (desktop): Import files into the Inbox
- [x] DOC-007 (desktop): Suggest a link for each Inbox file and rate it by matching signals
- [x] DOC-008 (desktop): Accept, skip, file by hand, accept all strong matches and undo
- [x] DOC-009 (desktop): Correct what was read from an Inbox file
- [x] DOC-010 (desktop): Flag renewal, end, expiry and revalue dates, and count those needing review
- [x] DOC-011 (desktop): Open a file or show it in its folder
- [ ] DOC-012: Read text from files (OCR and PDF extraction)
- [ ] DOC-013: Watched folder and dropping files onto the window
- [ ] DOC-014: Raise a notification from a key date reminder
- [ ] DOC-015: Save documents and links in the ledger
- [ ] DOC-016: Documents in the terminal app

## For developers

Curious how Documents is structured in the codebase, or planning to change it? See the [Documents development documentation](development/documents.md).
