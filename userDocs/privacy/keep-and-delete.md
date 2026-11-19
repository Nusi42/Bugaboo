<!-- Written by `cargo xtask gen keep-rules` from `bb_core::keeping::KEEP_KINDS`. Do not edit. -->

# Keep and delete rules

How long Bugaboo keeps each kind of data at least, and who deletes it then. These are the
periods a new organization starts with. A period that has a setting is changed by an
administrator on *Organization settings › Privacy › Keep and delete rules*, never below
the shortest period that kind allows.

## Bugaboo deletes these by itself

| Kind of data | Kept for | Why |
|---|---|---|
| IP addresses of sign-ins | 7 days | Stopping attacks on signing in |
| Sign-ins | 24 hours | Staying signed in; a sign-in that ended is deleted at once |
| Sign-ups that were used or ran out | 24 hours | Saying that a sign-up was finished already |
| Links that confirm a new e-mail address | 1 hour | Changing the e-mail address only after the new one is proved; a link that was opened is deleted at once |
| Requests for agents | 7 days | Showing you what a program asked for and what you answered; a request waits a day for your answer and a day for its keys |
| Drafts | 30 days | Not losing text that was not saved |
| Notifications | 90 days | Not needed longer |
| Trash | 30 days | Time to bring back what was deleted by mistake; an issue whose time entries must still be kept stays |
| Unfinished uploads | 24 hours | Time to finish an upload |
| Background work that failed | 30 days | Seeing why it failed |
| Ended access of AI apps | 30 days | Seeing which app had access |
| Log of tool calls | 90 days | Seeing what the assistant and AI apps did |
| Away times | 1 year | Planning; nobody needs them longer |
| Security log | 365 days | Finding and explaining security problems |
| Backups | No fixed end; about 28 days while a backup is created every day | Getting everything back after a loss; the newest backups of the days and weeks the settings of backups say are kept whatever their age, and a backup created before a restore is kept as long as the security log |
| Data exports | 7 days | Giving a person the file of Download my data; the record of the file goes with it |

## An administrator deletes these after their period

| Kind of data | Kept for | Why |
|---|---|---|
| Working hours | 2 years | German working-time recording |
| Time on issues | 10 years | Accounting; your organization sets the period |
