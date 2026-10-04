# Glossary — Bugaboo Words

Status: **Draft for review** · This file is the **single source of truth for naming**.

Rules:

1. **Plain language always.** Write for someone less technical than the average person.
   Use a technical word only if it is genuinely common outside software tools (for example
   *issue*, *workflow*, *dashboard*, *passkey*).
2. **Don't copy other vendors' naming.** Bugaboo has its own, plain vocabulary.
3. **The UI, user docs, API names and code all use these words.** A new concept gets an
   entry here *before* it is named anywhere else.
4. The "Don't say" column feeds the terminology check in the hooks
   (coding-standards §7). Those words fail the check when they appear in the message catalog.

---

## Work items

| Say (UI term)            | Meaning                                                            | Don't say                          |
|--------------------------|--------------------------------------------------------------------|------------------------------------|
| **Issue**                | Anything being tracked: a bug, a task, an idea                     | ticket, work item                  |
| **Issue ID**             | The readable ID of an issue: the project's short name plus a counting number (`WEB-142`) or a random code (`WEB-K7Q2X`), chosen per project | issue key, ticket number |
| **Counting / Random IDs**| The two ways a project gives out issue IDs. Random IDs don't reveal how many issues exist | sequence, auto-increment (in UI) |
| **Sub-project**          | A project inside another project; it inherits settings and permissions unless changed | child project, component, space |
| **Short name**           | The letters that start a project's issue IDs (`WEB`)               | project key                        |
| **Issue type**           | What kind of issue it is. Defaults: *Bug, Task, Feature, Goal*     | —                                  |
| **Goal**                 | A big piece of work that groups other issues under it              | epic, initiative, story            |
| **Sub-issue / parent**   | An issue that belongs under another one                            | sub-task, child story              |
| **Related issues**       | Issues connected to this one                                       | issue links (as UI label)          |
| **Relation**             | One connection between two issues                                  | link, edge                         |
| **Kind of relation**     | What the connection means. Each kind has a name for both directions: *blocks* / *is blocked by*, *duplicate of* / *has duplicate*, *causes* / *is caused by*, *related to* | link type |
| **Relation map**         | A picture of everything connected to an issue, step by step        | graph, network, link cloud, dependency graph |
| **Steps away**           | How many relations lie between two issues on the map               | depth, hops, degrees               |
| **Assigned to**          | Who is working on it                                               | assignee                           |
| **Reported by**          | Who created it                                                     | reporter                           |
| **Follow / followers**   | Get notified about changes to an issue or document                 | watch, watchers                    |
| **Release**              | A planned version of the product, e.g. *2.4*                       | fix version, version (as object)   |
| **Found in release**     | The release where a bug was seen                                   | affects version                    |
| **Planned for release**  | The release that should contain the fix or feature                 | fix version, target version        |
| **Visibility**           | Who may see a single issue. *Restricted* issues are visible only to chosen groups | issue security level |
| **Tags**                 | Free words anyone can add (`login`, `slow`)                        | —                                  |
| **Labels**               | A fixed list of words managed by admins                            | components                         |
| **Fields**               | The pieces of information on an issue. *Own fields* are those an organization adds | custom field (UI), attribute |

## Statuses and workflows

| Say                      | Meaning                                                            | Don't say                          |
|--------------------------|--------------------------------------------------------------------|------------------------------------|
| **Status**               | Where an issue stands right now (*Open*, *Fixing*, *Testing*, *Done*) | state                           |
| **Status group**         | Every status belongs to one of: *Not started*, *In progress*, *Done* | status category, To Do         |
| **Workflow**             | The statuses an issue type can have and the allowed changes between them | —                            |
| **Status change**        | Moving an issue from one status to another                         | transition                         |
| **Who can do it**        | Rule: which people may make a status change                        | condition                          |
| **Only if …**            | Rule: something that must be true first                            | condition                          |
| **Must be filled in**    | Rule: fields that need a value first                               | validator                          |
| **Ask for**              | Fields shown in a small form during the status change              | transition screen                  |
| **Then automatically …** | Actions that run after the change (assign, notify, fill a field)   | post-function                      |
| **Automations** (later)  | Rules that run on a schedule or event                              | —                                  |

## Finding and showing issues

| Say                      | Meaning                                                            | Don't say                          |
|--------------------------|--------------------------------------------------------------------|------------------------------------|
| **Search**               | Finding issues. *Simple search* uses filter buttons; *Advanced search* is typed text | query (in UI)    |
| **Filter**               | One condition in a search, shown as a button (`Status: not Done`)  | clause, predicate (in UI)          |
| **Saved search**         | A search plus how its results are shown, saved under a name        | view, filter (as saved object)     |
| **Search language**      | The text you type in Advanced search                               | any "xQL" name                     |
| **Show as**              | How results are shown: *List*, *Board*, *Timeline*, *Calendar*, *Map* | view type                       |
| **Board**                | Issues as cards in columns                                         | kanban (in UI), agile board        |
| **Rows** (on a board)    | Splitting a board horizontally by a field                          | swimlanes                          |
| **Card limit**           | Maximum cards a column should hold                                 | WIP limit                          |
| **Timeline**             | The Gantt chart (UI tab name; help text says "Gantt chart")        | roadmap                            |
| **Calendar**             | Issues shown on a month or week calendar by due date               | agenda (as tab name)               |
| **Map**                  | *Show as* option that draws search results and their relations     | graph view                         |
| **Must finish first**    | Dependency: this issue can't start until another finishes. Other kinds: *start together*, *finish together*, *finish when it starts* | finish-to-start, FS, SS, FF, SF (in UI) |
| **Deadline drivers**     | The chain of issues that sets the end date; any delay moves the end | critical path (in UI)             |
| **Milestone**            | An important date on the timeline                                  | —                                  |
| **Dashboard**            | A page of boxes showing searches, numbers and charts               | —                                  |
| **Quick search**         | The search box for everything, `Ctrl+K`                            | command palette, omnibox           |

## Time

| Say                      | Meaning                                                            | Don't say                          |
|--------------------------|--------------------------------------------------------------------|------------------------------------|
| **Timer**                | Start/stop clock on an issue                                       | stopwatch                          |
| **Time entry**           | A recorded amount of time on an issue                              | worklog                            |
| **Estimated time**       | How long it was expected to take                                   | original estimate                  |
| **Time left**            | How long it still needs                                            | remaining estimate                 |
| **Time spent**           | Total of all time entries                                          | logged work                        |
| **Timesheet**            | Your week of time entries in a grid                                | —                                  |
| **Working hours**        | When you worked on a day: start, end and breaks. Separate from time on issues | attendance, time clock     |
| **Work day**             | One day of working hours                                           | shift (unless the organization works in shifts) |
| **Start work day / End work day** | Buttons that record when work starts and ends             | clock in, clock out, punch in      |
| **Break**                | A pause that doesn't count as working time                         | —                                  |
| **Working-time rules**   | Legal limits checked on working hours (for example at most 10 hours a day) | compliance engine        |
| **Activity**             | What kind of work a time entry was: *Development, Testing, Documentation, Meetings, Support* | work type, category, task type |
| **Cost center**          | The accounting unit that pays for the work (Kostenstelle)          | cost object, account code          |

## Documents

| Say                      | Meaning                                                            | Don't say                          |
|--------------------------|--------------------------------------------------------------------|------------------------------------|
| **Documents**            | A project's pages and files                                        | space, wiki, library               |
| **Page**                 | A document written in Bugaboo                                      | article                            |
| **File**                 | An uploaded document (PDF, image …)                                | asset, blob (in UI)                |
| **Attachment**           | A file added to an issue or comment                                | —                                  |
| **Version**              | A saved state of a page or file                                    | revision                           |
| **Lock for editing**     | Stop others from changing a file while you work on it              | check out / check in               |
| **… is editing**         | Shown when someone else is changing a long text right now (*"Ana is editing the description"*) | locked by, checked out by |
| **Edit anyway**          | Take over a long text someone else is editing; they keep their unsaved text | force unlock, steal lock |
| **Saving… / Saved / Not saved yet** | The three save states shown next to a change            | syncing, persisted, dirty          |
| **Compare and save**     | See your text next to the newer version, then choose what to keep  | merge conflict, resolve conflict   |
| **Trash**                | Deleted items, restorable for a while                              | recycle bin                        |

## People and access

| Say                      | Meaning                                                            | Don't say                          |
|--------------------------|--------------------------------------------------------------------|------------------------------------|
| **Group**                | A set of people                                                    | team (as object)                   |
| **Role**                 | A named set of permissions. Defaults: *Administrator, Project lead, Team member, Contributor, Viewer, Guest* | — |
| **Permission**           | One thing a role allows                                            | grant, capability (in UI)          |
| **Hidden / Can see / Can change** | The three levels of field permissions                     | none / read / write (in UI)        |
| **Check access**         | See what someone can do, and why                                   | effective permissions              |
| **See as …**             | Look at Bugaboo as another person or role sees it                  | impersonate, preview as            |
| **Bot account**          | An account for automation, not a person                            | service account                    |
| **Access key**           | A secret for scripts and apps to sign in as you                    | personal access token, PAT         |
| **Connected apps**       | Apps you allowed to use your account, including AI apps            | OAuth clients                      |
| **Two-step sign-in**     | A second check after the password (code app or passkey)            | MFA, 2FA, TOTP (in UI)             |
| **Backup codes**         | One-time codes for when the second step is not available           | recovery codes                     |
| **Setup code**           | The one-time code printed by the server for first setup            | setup token                        |
| **Security log**         | The record of who changed what, which cannot be altered            | audit trail                        |
| **Download my data**     | Get a copy of everything Bugaboo stores about you                  | data subject access request, DSAR  |
| **Remove a person**      | Delete someone's account and replace their name with *Former user* plus a random code (*Former user K7Q2*) | anonymize, pseudonymize (in UI) |
| **Who sees people**      | Per role: show people's *Names*, only their *Role* ("a Team member"), or nothing (*Hidden*: "Someone") | identity masking, pseudonymization (in UI) |
| **Someone**              | Shown instead of a name when the viewer may not see who it was    | anonymous, redacted                |
| **Keep and delete rules**| How long each kind of data is kept before it is deleted automatically | retention policy                |

## Settings and appearance

| Say                      | Meaning                                                            | Don't say                          |
|--------------------------|--------------------------------------------------------------------|------------------------------------|
| **My settings**          | Personal preferences                                               | profile settings, preferences      |
| **Project settings**     | Settings of one project                                            | project admin                      |
| **Organization settings**| Settings for the whole Bugaboo server                              | admin console, instance settings   |
| **Layout**               | Which fields appear where: *issue layout*, *new-issue form*, *card layout* | screen, screen scheme      |
| **Theme**                | Colors and fonts. *Match my device*, *Light*, *Dark*               | skin                               |
| **Assistant**            | The built-in AI helper                                             | copilot, agent (in UI)             |
| **Suggestions**          | Values the assistant proposes, shown for you to accept or reject   | triage                             |
| **AI apps (MCP)**        | Outside AI tools connected through MCP                             | —                                  |
| **Written with the assistant** | Label on text the assistant wrote or proposed; also stored with the text | AI-generated content (as label) |
| **Self-hosted**          | Bugaboo running on the customer's own server                       | on-prem, on-premises, enterprise edition |
| **Hosted by Bugaboo**    | Bugaboo running for the customer on our servers in the EU          | SaaS, cloud edition                |
| **Notifications**        | Messages about things you follow or that involve you               | inbox, alerts                      |

## Internal terms (docs and code only, never in the UI)

| Term                | Meaning                                                               |
|---------------------|-----------------------------------------------------------------------|
| Policy engine       | The server component that makes every permission decision             |
| Instance            | One installed Bugaboo server                                          |
| AST                 | The parsed form of a search, shared by Simple and Advanced search     |
| Design tokens       | Named theme values (`bg.surface`), the only way code uses colors and sizes |
| Message catalog     | The single file of all user-facing text                               |
| Presentation        | How a saved search is shown (list, board, timeline, calendar)         |
| Sold model          | Customers run Bugaboo themselves (UI: *Self-hosted*)                  |
| Served model        | We run Bugaboo for customers (UI: *Hosted by Bugaboo*)                |
| Control plane       | Our separate system that creates, updates, backs up and monitors served instances; it holds no customer data |
| Operator            | Us, when running served instances; our staff have no default access to customer data |
| Manufacturer        | Us, as the maker of the software under the CRA, in both models        |
