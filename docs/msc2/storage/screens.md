# Storage screen sketches

P21.1 · October 17, 2026 · Awaiting Cameron's visual review.
Static wireframes, not application UI. Every name, size, date and outcome below
is illustrative. Read with the [inventory contract](contract.md). Brackets mark
controls, `>` a navigable row, `[x]` a selected checkbox. Right columns align;
quiet secondary lines sit below the primary label. Use the existing Settings
sheet and S0 tokens described in the contract, without adding a second sidebar.

## 1 — Saved computers

First read: computers and usage. Second: capacity and freshness. Third: coverage.
No network connections start merely because this view opens.

```text
Settings / Storage                                            [Close]

Storage
MSC files on your saved computers

Computer                                  MSC usage        Available
This computer                              43.0 GiB       118.0 GiB >
  Scanned Oct 17, 10:42 · Logical size
Basement server                            81.2 GiB         2 volumes >
  Connected · Scanned Oct 17, 10:20
Cabin                                      19.4 GiB         Unknown >
  Offline · Last scanned Oct 12, 18:03 · Historical results
Build machine                               Unknown        Unknown >
  Not scanned · Select this computer to refresh

Sizes describe known MSC files. Partial results are labelled below each name.
```

This computer remains present with no saved agent. With its agent offline it
reads “At least 0.8 GiB · Desktop measured; agent unavailable.” If no saved remote
computers exist, show the local row and “No other computers saved.” Refresh is
inside the chosen computer, never a default “scan all” action.

## 2 — Computer inventory

First read: total and computer identity. Second: volume availability and date.
Third: categories and focused actions. Only this screen has the proportional bar.
The neutral bar's segments match the category totals, not all disk usage.

```text
[‹ Computers]  This computer                                   [Refresh]

43.0 GiB measured
Logical size · Scanned Oct 17, 10:42 · Complete known-root coverage
[============================|=|==|===|=====]
 Servers                  Desktop Agent Tools Cache  (labels below bar)

Macintosh HD                  118.0 GiB available of 465.0 GiB
  Capacity checked Oct 17, 10:42

Servers                                            32.0 GiB >
Desktop app                                         0.8 GiB >
Agent                                               1.2 GiB >
Shared tools & runtimes                             3.0 GiB >
Cache & temporary files                             6.0 GiB >

File sizes may differ from disk space released by removal.
```

For multiple volumes replace the capacity block with one row per volume, e.g.
“Macintosh HD · 118 GiB available” and “Server SSD · 40 GiB available.” Each root
lists its volume in detail. If coverage is partial, title becomes “At least
43.0 GiB measured” and an inline explanation precedes categories. Allocated bytes
are a secondary labelled measure only when supported; do not mix bases in bar.

## 3 — Servers and server detail

```text
[‹ This computer]  Servers
32.0 GiB · Logical size · Scanned Oct 17, 10:42

Survival                                            25.0 GiB >
  Java · Stopped · Server SSD
Creative                                             7.0 GiB >
  Bedrock · Running · Macintosh HD

[‹ Servers]  Survival
This computer · Java · Stopped
25.0 GiB · Logical size · Server SSD

Worlds                                              15.0 GiB >
Backups                                              8.0 GiB >
Mods & server files                                  1.5 GiB >
Logs & other files                                   0.5 GiB >

[Open folder]                              [Review server removal]
```

Worlds lists the active world and stored slots separately with names, size and
Active/Stored text. Selecting one opens its individual detail and removal review;
there is no multi-world cart. Unknown slot metadata is counted as protected data.
Mods/server files and logs are inspectable; they do not acquire a generic delete
button. Unrecognized files are labelled “Unrecognized · Retained.” A running
server uses existing lifecycle controls and explains why removal is unavailable.
Remote detail replaces Open folder with a labelled path and Copy path, naming
that computer. Local reveal is absent when the directory cannot be verified.

## 4 — Category selection

```text
[‹ This computer]  Cache & temporary files
6.0 GiB · Logical size · Scanned Oct 17, 10:42
Select inactive files to review. Active or unrecognized files are retained.

[x] Saved map cache                                  2.0 GiB
      Survival · Rebuildable saved-terrain output
[x] Old downloads                                    1.0 GiB
      No current installation or active transfer references
[ ] Import staging                                   3.0 GiB
      Protected · Import is using these files (checkbox disabled)

2 selected · 3.0 GiB measured                    [Review selected]
```

Entry actions “Clear map cache” and “Remove old downloads” preselect only that
eligible kind and open review, never execute immediately. Mixed cache kinds may
share the cache review, with dependencies shown per item. No Select all includes
protected rows. Selection does not survive a changed root or computer identity.

Backups uses the same flat selection pattern scoped to This computer / Survival
/ Backups, with date, automatic/manual label and size for each backup. “Review
backups” opens this list. Batch selection stays within this category/computer;
existing per-server retention rules still apply to every selected backup.

Desktop app, Agent and Shared tools detail use identical name/size rows with
Current installation, Required for rollback or Used by Survival protection text.
Show the packaged payload explanation: “The desktop app includes its bundled
agent and tools. Separately installed copies appear in Agent and Shared tools.”
Offer existing agent/update management links only when available. Whole-app or
agent removal links to the separate complete-uninstall flow; no cleanup shortcut.

## 5 — Cache removal review

```text
[‹ Cache selection]  Clear selected files
This computer · Cache & temporary files

Saved map cache                                      2.0 GiB
  Survival · Maps rebuild when next requested; generation takes time.
Old downloads                                        1.0 GiB
  No active installation depends on these files.

2 items · 3.0 GiB logical size
Space released may be lower because files can share disk blocks.
Checked just now · Review expires in 5 minutes

[Cancel]                                      [Clear selected files]
```

Refreshing ownership at preview can retain an item formerly selectable; explain
it and require a revised review rather than silently changing the deletion set.

## 6 — Backup removal review (separate from cache)

```text
[‹ Backups]  Remove selected backups
This computer · Survival · Backups

Oct 15, 22:00 · Automatic                             2.0 GiB
Oct 16, 22:00 · Automatic                             2.0 GiB

These restore points will be permanently removed.
The current world remains. Existing backup-retention checks still apply.
2 backups · 4.0 GiB logical size · Space released is an estimate.

<existing backup confirmation control, if required by the service>
[Cancel]                                  [Remove selected backups]
```

If retention checks refuse the selection, show the reason with Return to backups;
never relax the rule or silently add/remove a backup. No map-cache choices here.

## 7 — World removal review

```text
[‹ Worlds]  Remove world
This computer · Survival

Old spawn · Stored world                             3.0 GiB
<existing world-deletion consequences and confirmation>
Backups: show the existing service's actual retained/affected scope.

3.0 GiB logical size · Space released is an estimate.
[Cancel]                                             [Remove world]
```

The existing service determines whether an active world is removable and which
stopped-state/safety requirements apply. If refused, the review shows the actual
reason and a return action. Never invent an automatic world switch or stop.
Actual domain consequences replace the placeholder above during implementation;
Storage must not paraphrase away a required existing confirmation.

## 8 — Server removal review

```text
[‹ Survival]  Remove server
This computer · Survival · Java · Stopped

Worlds (active and stored)                           15.0 GiB
Backups                                              8.0 GiB
Mods & server files                                  1.5 GiB
Logs & other files                                   0.5 GiB

25.0 GiB contained data · Logical size
<existing server-removal consequences, scope choice and confirmation>
Shared Java used by Creative will be retained.

[Cancel]                                            [Remove server]
```

Name contained worlds and backup count in an expandable plain list; show actual
scope chosen by the existing removal flow, including any retain-files option.
Contained totals reflect exclusions/shared references. Unreadable contained data
is explicitly unknown and cannot be presented as a complete deletion estimate.
A required stop is explicit and does not start during preview. No world, backup
or cache review can silently become this review.

## 9 — Exceptional and in-progress states

All messages appear inline in the relevant flat list; no extra status cards.

| State | Visible treatment and available action |
|---|---|
| First scan/loading | “No scan yet” then “Scanning This computer · 12,400 entries checked · Worlds”; Cancel. Unknown total, no fabricated percent or zero. Cached data stays labelled Previous scan while new progress is shown. |
| Partial | “At least 43.0 GiB · 2 sources unreadable”; expand source/reason rows. Refresh. Known entries remain visible; affected actions blocked until revalidated. |
| Offline | “Cabin · Offline · Last scanned Oct 12, 18:03”; historical rows visible, removal controls disabled with “Reconnect and refresh before reviewing removal.” Refresh attempts only this chosen computer. |
| Permission denied | “Storage requires administrator permission on Basement server”; no unrelated path names. If only a root is unreadable, keep authorized totals with a source-level issue. Local desktop coverage remains separately labelled. |
| Empty | “No backups for Survival” after a complete authorized scan; no removal action. Unscanned, missing and denied sources never use this empty state. |
| Changed since scan | “These files changed since the scan. Refresh and review again.” Clear invalid selection and disable confirm; show which identities changed without substituting targets. |
| Unsupported agent | “Storage inventory unavailable on this agent”; existing agent-management action, local desktop coverage and separately supported map cleanup remain reachable. |
| Scan cancelled/budget reached | “Partial scan · Cancelled” or named limit, observation date, known totals and Refresh. Do not overwrite the last complete scan without retaining its dated context. |
| Cleanup running | Original computer/category, completed-item count and current action; Cancel remaining. No total-space-freed counter. Navigation never rebinds the operation. |

## 10 — Cleanup result

```text
[‹ This computer]  Cleanup partially completed
This computer · Cache & temporary files

Removed     Saved map cache                          2.0 GiB
Retained    Old downloads                            1.0 GiB
              Now referenced by an installation
Failed      Renderer cache                           Unknown
              Permission denied; files may remain

2.0 GiB logical file bytes removed. This is not measured disk space freed.
Macintosh HD · 119.1 GiB available · Checked Oct 17, 10:49

[Refresh inventory]                                  [Done]
```

On cancellation, list not-attempted items and say earlier removals remain removed.
On disconnect/restart, say “Result unconfirmed” for unknown items and reconcile
the original operation on reconnect before offering a fresh review. Never retry
a deletion automatically. A fresh review includes only explicitly reselected
remaining eligible items; failures do not disappear behind a success toast.

## Static design review

First read, hierarchy, flat containment and S0 palette are specified above.
No informational icons, dots, accent rails, nested cards, gradients, shadows or
motion occur in these wireframes. They are static text sketches, so pixel layout,
keyboard behavior and actual platform rendering remain for P21.7 and Cameron's
visual verification. Cameron's P21.1 sketch/contract acceptance is pending; these
examples do not claim runtime or deletion acceptance.
