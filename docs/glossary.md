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
   (coding-standards §7). Those words fail the check when they appear in the message catalog,
   in the user docs or in API names (paths, schema names, property names, enum values, operation IDs, parameter names); a word inside a
   longer word does not count. A bracket at the end of a cell says where an entry applies:
   `(in UI)` and its kind are checked, the others (`(as object)`) are left to review. A word
   that is right in another sense gets a reason in `tools/checks/terms-exemptions.toml`.
   Enforced by: TXT-TERMS.

---

## Work items

| Say (UI term)            | Meaning                                                            | Don't say                          |
|--------------------------|--------------------------------------------------------------------|------------------------------------|
| **Issue**                | Anything being tracked: a bug, a task, an idea                     | ticket, work item                  |
| **Issue ID**             | The readable ID of an issue: the project's short name plus a counting number (`WEB-142`) or a random code (`WEB-K7Q2X`), chosen per project | issue key, ticket number |
| **Counting / Random IDs**| The two ways a project gives out issue IDs. Random IDs don't reveal how many issues exist | sequence, auto-increment (in UI) |
| **Sub-project**          | A project inside another project; it inherits settings and permissions unless changed | child project, component, space |
| **Short name**           | The letters that start a project's issue IDs and its address (`WEB`, `/p/WEB`): 2 to 10 capital letters or digits, starting with a letter, unique in the organization. It can be changed; the old one keeps working and stays with the project | project key |
| **Top level**            | Where a project that is inside no other project sits               | root project (in UI)               |
| **Move** (a project)     | Put a project inside another one, or at the top level. Before it happens, Bugaboo says who gains or loses access | re-parent, reparent (in UI) |
| **Overview** (of a project) | The page of a project at `/p/WEB`: its name, short name, where it sits and its sub-projects | dashboard (for this page), home of a project |
| **Preview** (of a text)  | How a description, a comment or a long text will look once it is saved: the *Preview* tab next to the text box of the editor. It is made by the same program that shows the saved text, so it is what others will see. Not the *Preview* of a move | live view, render |
| **Preview** (of a move)  | What Bugaboo shows before a move happens: in plain sentences, who gains or loses access or changes role, and which fields and people information change for whom. The move asks for the version of the preview that was read | dry run, simulation, impact analysis |
| **Where you are**        | The line at the top that names the page you are on and the pages above it (*ACME › Website › Project settings*). It names only what you may see | breadcrumb (in UI) |
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
| **Choice**               | One of the values a *Pick one* or *Pick several* field offers (*Critical*, *Major*), with its color and icon | enum value, list item (in UI) |
| **Combine with another choice** | Move every issue from one choice, tag or label to another, then remove the first one | merge (in UI) |
| **Retire**               | Stop offering a choice, label, issue type or field for new issues while old issues keep it | archive, deprecate (in UI) |
| **Look** (of a field)    | How a field's value is shown: *Badge, Pill, Plain text, Progress bar, Avatar* | display style, renderer |
| **Pick one group**       | A group of labels of which an issue can have only one (*area: login* or *area: billing*) | exclusive group, mutually exclusive |
| **Comment**              | What people write under an issue; it can be changed later, and its earlier versions are kept | note (for this), reply |
| **Mention**              | Naming a person, group or role in a text with `@`, so they find it under *Mentions* | ping, tag (for people) |
| **History** (of an issue) | The comments and changes of an issue in time order, with the tabs *Everything · Comments · Changes*; changes to fields you can't see are left out | activity (for this), audit log (in UI), timeline (for this) |
| **Change many issues**   | Select several issues and change them together, after a preview that says what happens to each one | bulk edit, mass update |
| **Add a field**          | The quiet line on an issue that offers the fields you may fill in that are still empty | — |

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
| **Starting workflow**    | The workflow every issue type uses until an organization makes its own (M4): *Open*, *In progress*, *Done*, and any status can follow any other | default workflow (in UI) |
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
| **Focus view**           | An open item takes the main area; the menu shrinks to icons and a slim list stays on the left | full-screen mode, detail view |
| **Side panel**           | A panel next to a list in which a simple item opens, so the list stays in view | drawer, flyout, split view |

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
| **Draft**                | Text you have written but not saved yet. Bugaboo keeps it privately, in your browser and on the server, until you save or throw it away, at most 30 days | autosave (as a noun), unsaved changes |
| **Ask … to finish**      | Ask the person who is editing a text to save it, so you can edit it | request unlock |
| **Keep Ben's / Use mine** | The two answers when somebody changed a short value (a status, a person, a date) just before you did | resolve conflict |
| **Show changes**         | See what somebody changed in a text after you opened it              | diff (in UI)                       |
| **Trash**                | Deleted items, restorable for a while                              | recycle bin                        |

## People and access

| Say                      | Meaning                                                            | Don't say                          |
|--------------------------|--------------------------------------------------------------------|------------------------------------|
| **Group**                | A set of people                                                    | team (as object)                   |
| **Role**                 | A named set of permissions, given to people or groups for the whole organization or on a project. Every organization starts with *Administrator, Project lead, Team member, Contributor, Viewer, Guest* | — |
| **Starting role**        | One of the six roles every organization starts with. It is a copy that can be renamed, changed or deleted | built-in role, default role, system role, predefined role (in UI) |
| **Reset to the starting permissions** | Give a starting role back the permissions it had at the start | factory reset, restore defaults (in UI) |
| **Who has which role**   | The list, for the whole organization or one project, of the people and groups and their role there. Rows from a parent project say so (*from ACME*) | role assignments, access control list, ACL (in UI) |
| **Where it counts**      | Where a role given to someone applies: *Whole organization*, *Only this project*, or *This project and its sub-projects* | global, subtree, recursive (in UI) |
| **No access**            | Set on a project for a person or group: they see and can do nothing in this project and its sub-projects, whatever role they have from higher up or through a group there. A role given on a sub-project lower down still counts there | deny, denied (in UI) |
| **Used by**              | On a group or a role: where it is given (*Used for the whole organization and in 4 projects you can see*). It counts only what the viewer may see | usage, references, dependents (in UI) |
| **The role that takes its place** | When a role that somebody has is deleted: the role everybody with it gets instead, in the same place and with the same reach. In the API and the code: `replacement` | reassign, migrate (in UI) |
| **Permission**           | One thing a role allows                                            | grant, capability (in UI)          |
| **Hidden / Can see / Can change** | The three levels of field permissions                     | none / read / write (in UI)        |
| **Check access**         | See what someone can do in a project, and why, in sentences (*"Ana is in the group Backend, which has the role Team member in ACME"*) | effective permissions |
| **See as …**             | Look at Bugaboo as another person or role sees it                  | impersonate, preview as            |
| **Bot account**          | An account for automation, not a person                            | service account                    |
| **Access key**           | A secret for scripts and apps to sign in as you. It can do at most what you can, and it always runs out | personal access token, PAT |
| **See only / See and change / Everything I can do** | The three choices of what an access key may do; single permissions sit under *More options* | read-only, read-write, full access (in UI) |
| **Connected apps**       | Apps you allowed to use your account, including AI apps            | OAuth clients                      |
| **Sign in / Sign out**   | Open or end your use of Bugaboo with your password or a passkey    | log in, log out, login, logout     |
| **Sign out everywhere**  | End all your sessions at once, this one included                   | revoke all tokens, global logout   |
| **Session**              | One place where you are signed in, for example one browser. *Where you're signed in* lists them and lets you end any | login session, token (in UI) |
| **Where you're signed in** | The list of your sessions, with the browser, the system and when each was last used | active sessions, devices, logins |
| **Sign up**              | Make your own account without an invitation, when an administrator allows it | register, registration, self-service signup |
| **Invitation**           | A link an administrator makes so that one person can make their account; it works once and runs out after 7 days | invite code, signup token |
| **Passkey**              | A way to sign in with your device (fingerprint, face or screen lock) instead of typing a password; can also be the second step | security key, WebAuthn, FIDO (in UI) |
| **Code app**             | An app on your phone that shows a new six-digit code every 30 seconds; the first kind of second step | authenticator, authenticator app, OTP app, TOTP (in UI) |
| **Second step**          | The check after the password: a code from the code app, a backup code or a passkey | verification step, challenge, factor |
| **Public address**       | The web address people type to reach Bugaboo, set by the person who runs the server; passkeys belong to its host name | public URL, base URL, external URL |
| **Two-step sign-in**     | A second check after the password (code app or passkey)            | MFA, 2FA, TOTP (in UI)             |
| **Backup codes**         | One-time codes for when the second step is not available           | recovery codes                     |
| **Setup code**           | The one-time code printed by the server for first setup            | setup token                        |
| **Security log**         | The record of who changed what, which cannot be altered            | audit trail                        |
| **Download my data**     | Get a copy of everything Bugaboo stores about you                  | data subject access request, DSAR  |
| **Remove a person**      | Delete someone's account and replace their name with *Former user* plus a random code (*Former user K7Q2*) | anonymize, pseudonymize (in UI) |
| **Who sees people**      | Per role and per kind of people information: show people's *Names*, only their *Role* ("a Team member"), or nothing (*Hidden*: "Someone") | identity masking, pseudonymization (in UI) |
| **Kind of people information** | One of the places a person can be shown: *Assigned to, Reported by, Last changed by*, comment and history authors, *"… is editing"*, followers, time entries, mentions, pictures of people, *Project members* | identity field, people field (in UI) |
| **Project members**      | The kind of people information for lists of who works in a project. People who manage people or roles always see the names there | roster, participants (in UI) |
| **Someone**              | Shown instead of a name when the viewer may not see who it was    | anonymous, redacted                |
| **Keep and delete rules**| How long each kind of data is kept before it is deleted automatically | retention policy                |

## Settings and appearance

| Say                      | Meaning                                                            | Don't say                          |
|--------------------------|--------------------------------------------------------------------|------------------------------------|
| **My settings**          | Personal preferences                                               | profile settings, preferences      |
| **Project settings**     | Settings of one project                                            | project admin                      |
| **Organization settings**| Settings for the whole Bugaboo server                              | admin console, instance settings   |
| **Reset to default**     | Take away the value set at this level, so the value from above counts again | clear override (in UI) |
| **Set by your organization** | Shown on a setting an administrator locked: it can't be changed here, and the line says why | enforced (in UI) |
| **Layout**               | Which fields appear where: *issue layout*, *new-issue form*, *card layout* | screen, screen scheme      |
| **Spacing**              | How close together things are on screen: *Comfortable* or *Compact* | density (in UI) |
| **Less motion**          | Fewer moving effects on screen                                     | reduced motion (in UI)             |
| **Larger text**          | All text a step bigger                                             | zoom, font scale (in UI)           |
| **Appearance**           | The choices about how Bugaboo looks: theme, *Spacing* (*Comfortable* or *Compact*), *Larger text* and *Less motion* | display settings |
| **Keyboard shortcuts**   | Keys that do something faster; every shortcut also has a button, and a person can choose their own keys | hotkeys, keybindings |
| **Region**               | The choices about where you are: time zone, date format, number format and first day of the week | locale, localization, i18n (in UI) |
| **Start page**           | The page Bugaboo opens after you sign in                           | landing page, home screen (in UI)  |
| **Sign-in rules**        | The organization settings for how long people stay signed in and who must use two-step sign-in | authentication policy, session policy (in UI) |
| **Undo**                 | Taking back what was just done; offered for 10 seconds after an action that can be reversed | revert, rollback (in UI) |
| **Theme**                | Colors and fonts. Built in: *Daylight* and *Paper* (light), *Dusk* and *Midnight* (dark). *Match my device* uses your light or dark choice depending on the device setting | skin |
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
| Assignment          | One row of *Who has which role*: a person or group, a role or *No access*, and where it counts. Screens say "Ana has the role Team member in Website" |
| Permission catalogue | The one list of permissions in `bb-policy`, each with its sentence, topic, where it counts and kind |
| Field catalogue     | The one list of built-in fields in `bb-core` that field permissions apply to |
| `PersonRef`         | How every person in an answer is shown, worked out for the viewer: name, role label, *Someone* or *Former user* |
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
