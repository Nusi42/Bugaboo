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
   `(in UI)` and its kind are checked, the others (`(as object)`) are left to review. Code names
   (the types, functions, constants, modules, enum variants, files and folders that Rust and
   TypeScript declare here, and the message keys) are read for an entry only when its bracket says
   `code` (`(in UI and code)`), because most entries are ordinary words of programming (*state*,
   *token*, *link*) that are right in code; a name is cut into words (`ManyChangeRun` reads *many
   change run*). A word that is right in another sense gets a reason in
   `tools/checks/terms-exemptions.toml`. Enforced by: TXT-TERMS.

---

## Work items

| Say (UI term)            | Meaning                                                            | Don't say                          |
|--------------------------|--------------------------------------------------------------------|------------------------------------|
| **Issue**                | Anything being tracked: a bug, a task, an idea                     | ticket, work item                  |
| **Issue ID**             | The readable ID of an issue: the project's short name plus a counting number (`WEB-142`) or a random code (`WEB-K7Q2X`), chosen per project | issue key, ticket number (in UI and code) |
| **Counting / Random IDs**| The two ways a project gives out issue IDs. Random IDs don't reveal how many issues exist | sequence, auto-increment (in UI) |
| **Sub-project**          | A project inside another project; it inherits settings and permissions unless changed | child project, component, space |
| **Short name**           | The letters that start a project's issue IDs and its address (`WEB`, `/p/WEB`): 2 to 10 capital letters or digits, starting with a letter, unique in the organization. It can be changed; the old one keeps working and stays with the project | project key |
| **Top level**            | Where a project that is inside no other project sits               | root project (in UI)               |
| **Move** (a project)     | Put a project inside another one, or at the top level. Before it happens, Bugaboo says who gains or loses access | re-parent, reparent (in UI) |
| **Overview** (of a project) | The page of a project at `/p/WEB`: its name, short name, where it sits and its sub-projects | dashboard (for this page), home of a project |
| **Preview** (of a text)  | How a description, a comment or a long text will look once it is saved: the *Preview* tab next to the text box of the editor. It is made by the same program that shows the saved text, so it is what others will see. Not the *Preview* of a move | live view, render |
| **Preview** (of a move)  | What Bugaboo shows before a move happens: a sentence on top that says how many people change, then, grouped by person, who gains or loses access or changes role and which fields and people information change for whom, ten people at a time with *Show all*. The move asks for the version of the preview that was read | dry run, simulation, impact analysis |
| **Preview** (of a change to many issues) | What Bugaboo shows before a rename, a combination or a change to ticked issues happens: a sentence on top (*118 issues will change, 2 will be skipped*), then each issue with what happens to it, ten at a time with *Show all*; an issue that is left alone says why. Made with the one component of the long preview | dry run, simulation |
| **Stopped** (a change to many issues) | The state of a change that goes on in the background and was ended before its last issue: by the person who asked, by somebody who manages people (the permission *Manage people* covers the things another person started), or because the person signed out everywhere or changed their password. What was changed stays, the rest is left alone. The answer of stopping holds the state only, never the counts | cancelled, aborted |
| **Where you are**        | The line at the top that names the page you are on and the pages above it (*ACME › Website › Project settings*). It names only what you may see | breadcrumb (in UI) |
| **Issue type**           | What kind of issue it is. Defaults: *Bug, Task, Feature, Goal*     | —                                  |
| **Offered in**           | Where an issue type or a label group is offered: lines for the whole organization or a project (and the projects inside it); a project follows the one above it until it has a line of its own. A project that does not offer a type does not show it in *New issue*; issues keep the type they have | enabled for, available in (as UI label) |
| **Goal**                 | A big piece of work that groups other issues under it              | epic, initiative, story            |
| **Sub-issue / parent**   | An issue that belongs under another one                            | sub-task, child story              |
| **Related issues**       | Issues connected to this one                                       | issue links (as UI label)          |
| **Relation**             | One connection between two issues                                  | link, edge |
| **Kind of relation**     | What the connection means. Each kind has a name for both directions: *blocks* / *is blocked by*, *duplicate of* / *has duplicate*, *causes* / *is caused by*, *related to* | link type |
| **Relation map**         | A picture of everything connected to an issue, step by step        | graph, network, link cloud, dependency graph |
| **Steps away**           | How many relations lie between two issues on the map               | depth, hops, degrees               |
| **Assigned to**          | Who is working on it                                               | assignee (in UI and code)          |
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
| **Combine with another choice** | Move every issue from one choice, tag or label to another, then retire the first one | merge (in UI) |
| **Retire**               | Stop offering a choice, label, issue type or field for new issues while old issues keep it | archive, deprecate (in UI) |
| **Look** (of a field)    | How a field's value is shown: *Badge, Pill, Plain text, Progress bar, Avatar* | display style, renderer |
| **Label group**          | A named group of labels (*area*, *platform*); a bare *Group* always means people | category (for labels) |
| **Label group to pick one from** | A label group of which an issue can have only one label (*area: login* or *area: billing*); the screens say *Pick one* on its switch | pick one group, exclusive group, mutually exclusive |
| **Comment**              | What people write under an issue; it can be changed later, and its earlier versions are kept | note (for this), reply |
| **Mention**              | Naming a person, group or role in a text with `@`, so they find it under *Mentions* | ping, tag (for people) |
| **History** (of an issue) | The comments and changes of an issue in time order, with the tabs *Everything · Comments · Changes*; changes to fields you can't see are left out | activity (for this), audit log (in UI), timeline (for this) |
| **Change many issues**   | Select several issues and change them together, after a preview that says what happens to each one | bulk, bulk edit, mass update (in UI and code) |
| **Add a field**          | The quiet line on an issue that offers the fields you may fill in that are still empty | — |

## Statuses and workflows

| Say                      | Meaning                                                            | Don't say                          |
|--------------------------|--------------------------------------------------------------------|------------------------------------|
| **Status**               | Where an issue stands right now (*Open*, *Fixing*, *Testing*, *Done*) | state                           |
| **Status group**         | Every status belongs to one of: *In progress*, *Not started*, *Done* (the order a list shows its groups in; sorting and comparing use another, see architecture 04 *search*) | status category, To Do         |
| **Workflow**             | The statuses an issue type can have and the allowed changes between them | —                            |
| **Status change**        | Moving an issue from one status to another                         | transition                         |
| **Who can do it**        | Rule: which people may make a status change                        | condition                          |
| **Only if …**            | Rule: something that must be true first                            | condition                          |
| **Must be filled in**    | Rule: fields that need a value first. The box is named *Customer (required)*, and a refusal says *Must be filled in* at that box | validator                          |
| **Ask for**              | Fields shown in a small form during the status change              | transition screen                  |
| **Starting workflow** (docs only, before M4) | The workflow every issue type used before M4: *Open*, *In progress*, *Done*, and any status can follow any other. Since M4 the UI says *starter workflow*: the workflows that come with Bugaboo, such as *Simple* | default workflow (in UI) |
| **Starter workflows**    | The workflows a new organization starts with, as copies it may change: *Simple*, *Bug* (with *Testing* and *Won't fix*) and *Development* (with *In review* and *Testing*) | template, scheme (in UI) |
| **Publish** (a workflow) | Make the changed workflow count; until then the changes are a draft only the editor sees | deploy, activate (in UI) |
| **Try it** (a workflow)  | Pick an issue and a person and see which status changes they would get, and why | simulate, dry run (in UI) |
| **Then automatically …** | Actions that run after the change (assign, notify, fill a field)   | post-function                      |
| **Automations** (later)  | Rules that run on a schedule or event                              | —                                  |

## Finding and showing issues

| Say                      | Meaning                                                            | Don't say                          |
|--------------------------|--------------------------------------------------------------------|------------------------------------|
| **Search**               | Finding issues. *Simple search* uses filter buttons; *Advanced search* is typed text | query (in UI)    |
| **Filter**               | One condition in a search, shown as a button (`Status: not Done`)  | clause, predicate (in UI)          |
| **Saved search**         | A search plus how its results are shown, saved under a name        | view, filter (as saved object)     |
| **Save search**          | Keep the search you have open, with its columns and order, under a name | save view, save filter      |
| **Save a copy**          | Keep an open saved search as a new one of your own, private at first | clone, fork (in UI)               |
| **Share** (a saved search) | The window that chooses the groups and projects a saved search is shared with; each person still sees only what they may see | —                                |
| **Share searches**       | The permission to share saved searches with a group or a project    | —                                  |
| **Mine** / **Shared with me** | The two lists of *Saved searches*: the ones you made, and the ones others shared with you | owned, inbox (in UI) |
| **Group by**             | Show the results of a search in groups, one per value of a field; an issue with several values (labels, tags, people) stands in the group of each, one without a value in *None* | aggregate, bucket |
| **Search language**      | The text you type in Advanced search                               | any "xQL" name                     |
| **Show as**              | How results are shown: *List*, *Board*, *Timeline*, *Calendar*, *Map* | view type                       |
| **Board**                | Issues as cards in columns                                         | kanban (in UI), agile board        |
| **Rows** (on a board)    | Splitting a board horizontally by a field                          | swimlanes                          |
| **Card limit**           | Maximum cards a column should hold                                 | WIP limit                          |
| **Timeline**             | The Gantt chart (UI tab name; help text says "Gantt chart")        | roadmap                            |
| **Calendar**             | Issues shown on a month or week calendar by due date               | agenda (as tab name)               |
| **Map**                  | *Show as* option that draws search results and their relations     | graph view                         |
| **Must finish first**    | Dependency: this issue can't start until another finishes. Other kinds: *start together*, *finish together*, *finish when it starts* | finish-to-start, FS, SS, FF, SF (in UI) |
| **Plan by effort**       | Let the timeline work out a bar's length from its estimate, its time factor and the person's work days, away times and public holidays | auto-schedule, resource levelling (in UI) |
| **Time factor**          | A value on an issue, for its assigned person: how much longer or shorter than the estimate it will take them (*×1.5*). The assigned person may always set it; it goes back to ×1 when the issue goes to another person | velocity, performance factor, speed factor (in UI) |
| **Plan warnings**        | What the timeline points out: *Away*, *Not enough days*, *Delay*, *Too much at once*, each with what can be done | clashes, overallocation (in UI) |
| **Keep and accept**      | The answer to a plan warning that leaves the plan as it is; the warning stays, faded | ignore, dismiss (in UI) |
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
| **Upload**               | Put a file into Bugaboo: drag it in, paste it or choose it; an interrupted upload goes on where it stopped | — |
| **Files** (on an issue)  | The block of an issue's attachments, with a preview of each image and PDF | attachments panel (in UI) |
| **Preview** (of a file)  | The small picture Bugaboo makes of an image or the first page of a PDF; other files have none and are only downloaded | thumbnail (in UI) |
| **Download**             | Save a copy of a file on your device; Bugaboo never opens other files inside the page | — |
| **Attachment**           | A file added to an issue or comment                                | —                                  |
| **Version**              | A saved state of a page or file                                    | revision                           |
| **Lock for editing**     | Stop others from changing a file while you work on it              | check out / check in               |
| **… is editing**         | Shown when someone else is changing a long text right now (*"Ana is editing the description"*); the screens and the user docs never say *lock* for it | locked by, checked out by |
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
| **Agent**                | An account that works on its own and is not a person: an AI app or a script. It has one responsible person and can do at most what that person can. Shown as *"Fixer, agent of Ada Brandt"* | bot, bot account, service account, AI worker (in UI) |
| **Agent key**            | The access key an agent signs in with; it belongs to the agent and always runs out | bot token, service token (in UI) |
| **Create agents that work for you** | The permission to create agents of your own                 | — |
| **Agents of the organization** (later) | Agents whose responsible party is the organization, with a role; webhooks and imports run as one | system bot (in UI) |
| **Access key**           | A secret for scripts and apps to sign in as you. It can do at most what you can, and it always runs out | personal access token, PAT |
| **See only / See and change / Everything I can do** | The three choices of what an access key may do; single permissions sit under *More options* | read-only, read-write, full access (in UI) |
| **Connected apps**       | Apps you allowed to use your account, including AI apps            | OAuth clients                      |
| **Sign in / Sign out**   | Open or end your use of Bugaboo with your password or a passkey    | log in, log out, login, logout     |
| **Sign out everywhere**  | End all your sessions at once, this one included                   | revoke all tokens, global logout   |
| **Session**              | One place where you're signed in, for example one browser. *Where you're signed in* lists them and lets you end any | login session, token (in UI) |
| **Where you're signed in** | The list of your sessions, with the browser, the system and when each was last used | active sessions, devices, logins |
| **Sign up**              | Ask for your own account without an invitation, when an administrator allows it and e-mail is on: an e-mail with a link goes to your address, and opening it lets you choose your name and password | register, registration, self-service signup |
| **Invitation**           | A link an administrator creates so that one person can create their account; it works once and runs out after 7 days | invite code, signup token |
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
| **Who sees people**      | Per role and per kind of people information: show people's *Names*, only their *Role* ("Someone (Team member)"), or nothing (*Hidden*: "Someone") | identity masking, pseudonymization (in UI) |
| **Kind of people information** | One of the places a person can be shown: *Assigned to, Reported by, Last changed by*, comment and history authors, *"… is editing"*, followers, time entries, mentions, pictures of people, *Project members*, *Work times* | identity field, people field (in UI) |
| **Away**                 | A stretch of whole or half days on which a person does not work. Bugaboo stores no reason; others see only *Away* | absent, absence, out of office, sick, on leave (in UI) |
| **Away times**           | A person's away stretches                                | absences, leave calendar (in UI) |
| **Work times**           | A person's usual work days, work hours, shares in projects and away times; also the kind of people information that says who may see them | availability, capacity (in UI) |
| **Enter work times for others** | The permission to enter usual work days, work hours and away times for other people | — |
| **Usual work days**      | The days of the week a person normally works, for example Monday to Thursday | work pattern, schedule (in UI) |
| **Work hours**           | How many hours a person works on a usual work day, from their work contract | capacity, FTE (in UI) |
| **Share in the project** | How much of a person's work hours belong to one project, for example 60 % | allocation, utilization (in UI) |
| **Public holidays**      | Days nobody in a region works, from a preset per country and, for Germany, per federal state | bank holidays (in UI) |
| **Project members**      | The kind of people information for lists of who works in a project. People who manage people or roles always see the names there | roster, participants (in UI) |
| **Someone**              | Shown instead of a name when the viewer may not see who it was    | anonymous, redacted                |
| **Keep and delete rules**| How long each kind of data is kept before it is deleted automatically | retention policy                |

## Settings and appearance

| Say                      | Meaning                                                            | Don't say                          |
|--------------------------|--------------------------------------------------------------------|------------------------------------|
| **My settings**          | Personal preferences                                               | profile settings, preferences      |
| **Project settings**     | Settings of one project                                            | project admin                      |
| **Organization settings**| Settings for the whole Bugaboo server                              | admin console, instance settings   |
| **Server status**        | Whether the Bugaboo server is running and healthy: its database, its certificate, its security log (the route `/api/v1/status`; not the status of an issue) | health page, health check page     |
| **Reset to default**     | Take away the value set at this level, so the value from above counts again; also for a workflow or a layout that a place has of its own (it goes back to the one that counts there without its own) | clear override, use the inherited … again (in UI) |
| **Default value**        | The value a setting has until somebody sets one at a level: the line under a setting says *Default value* when none is set; *Reset to default* brings it back | standard value, Bugaboo's standard value (in UI) |
| **Set by your organization** | Shown on a setting an administrator locked: it can't be changed here, and the line says why | enforced (in UI) |
| **Layout**               | Which fields appear where: on the *issue page*, in the *new-issue form*, on a *card* | screen, screen scheme      |
| **Section**              | A titled group of fields in a layout; sections with the same *tab* are shown together under it | panel, block (in a layout) |
| **Spacing**              | How close together things are on screen: *Comfortable* or *Compact* | density (in UI) |
| **Less motion**          | Fewer moving effects on screen                                     | reduced motion (in UI)             |
| **Larger text**          | All text a step bigger                                             | zoom, font scale (in UI)           |
| **Window**               | Where a form opens on a computer: it can be minimized, maximized, resized; on a phone it is a full screen | modal, popup (in UI) |
| **Minimize**             | Park a window as a bar at the bottom of the page; its draft waits there | — |
| **Maximize**             | Let a window fill the whole page, the menu included                | full screen (for a window)         |
| **Restore down**         | Give a maximized window its earlier size again                     | — |
| **Window size**          | The button in the title bar of a window with *Wider*, *Narrower*, *Taller* and *Shorter*, for people who cannot drag | resize handle menu |
| **Legend**               | The small list beside the list of issues: what the color of a row and its left edge mean; on a phone, and beside an open issue, it opens from a *Legend* button | key to colors, color guide |
| **Details**              | The name-and-value rows of an issue: who it is assigned to, its priority and the other fields | fields panel, attributes |
| **Not scheduled yet**    | The tray beside the timeline with the issues that have no dates yet | backlog (on the timeline)          |
| **Appearance**           | The choices about how Bugaboo looks: theme, *Spacing* (*Comfortable* or *Compact*), *Larger text* and *Less motion* | display settings |
| **Keyboard shortcuts**   | Keys that do something faster; every shortcut also has a button, and a person can choose their own keys | hotkeys, keybindings |
| **Region**               | The choices about where you are: time zone, date format, number format and first day of the week | locale, localization, i18n (in UI) |
| **Start page**           | The page Bugaboo opens after you sign in                           | landing page, home screen (in UI)  |
| **Sign-in rules**        | The organization settings for how long people stay signed in and who must use two-step sign-in | authentication policy, session policy (in UI) |
| **and**                  | The word between two names in a text for people (*Roles and permissions*, *E-mail and notifications*): never the sign & | & (in UI, a check refuses it) |
| **Create**               | The verb for bringing something new into Bugaboo, on every button and form. A button that opens a form says *Create an issue*, *Create a project*; the button that sends the form says *Create the issue*, *Create the project*, *Create the issue and start another* | made (in UI), making (in UI), make, add new (in UI and code) |
| **Undo**                 | Taking back what was just done; offered for 10 seconds after an action that can be reversed | revert, rollback (in UI) |
| **Theme**                | Colors and fonts. Built in: *Daylight* and *Paper* (light), *Dusk* and *Midnight* (dark). *Match my device* uses your light or dark choice depending on the device setting | skin |
| **Assistant**            | The built-in AI helper. Not an agent: it only proposes, and only to the person using it | copilot (in UI) |
| **Suggestions**          | Values the assistant proposes, shown for you to accept or reject   | triage                             |
| **AI apps (MCP)**        | Outside AI tools connected through MCP                             | —                                  |
| **Hand over** (to an agent) | Give one or several issues, by their IDs, to one of your agents to work on. The issue keeps its person | delegate, dispatch, assign to agent (in UI) |
| **Worked on by …**       | Shown on an issue while an agent works on it: *worked on by Fixer* | — |
| **Brief**                | What an agent is given with a hand-over: the issue's text, its steps, *Done when*, a note, and the answers to its questions | prompt, spec, ticket context (in UI) |
| **Done when**            | The part of a brief that says how to check the work is done        | acceptance criteria, definition of done (in UI) |
| **Standing rule**        | Lets one of your agents work on everything a saved search finds, by proposals or by changing a few named fields; it runs out | trigger, subscription, watch rule (in UI) |
| **Agent work**           | The record on an issue of what an agent did: the brief, its attempts, questions and the review. Kept apart from the comments | agent log, transcript (in UI) |
| **Attempt**              | One go of an agent at a handed-over issue: plan, steps, results    | iteration, job (in UI) |
| **Waiting for the agent / Agent working / Waiting for you / Ready for review / Agent gave up / Stopped** | The states of agent work. *Waiting for you* means a question or a plan waits for a person | needs input, idle (in UI) |
| **Approve plan**         | Let an agent start an attempt after reading its plan               | — |
| **Accept / Send back / Throw away** | The three answers of a review: keep the result; try again with a note; undo everything the attempts changed in Bugaboo | merge, reject, revert (in UI) |
| **Accept by itself when its checks pass** | A project setting for handed-over issues; off at the start | auto-merge, auto-accept (in UI) |
| **My agents**            | Your agents and all their work in one place, *Waiting for you* first | agent dashboard, fleet (in UI) |
| **Stop all my agents**   | Stop every one of your agents at once                              | kill switch (in UI) |
| **Run** (of an app, an agent or a change to many issues) | Everything one session of an AI app, one attempt of an agent or one *Change many issues* changed; shown with its progress and as a list, and undone in one step | batch, transaction (in UI) |
| **Written by an AI app / Written by an agent** | Labels on text written through MCP, stored with the text like *Written with the assistant* | AI-generated content (as label) |
| **Written with the assistant** | Label on text the assistant wrote or proposed; also stored with the text | AI-generated content (as label) |
| **Self-hosted**          | Bugaboo running on the customer's own server                       | on-prem, on-premises, enterprise edition |
| **Hosted by Bugaboo**    | Bugaboo running for the customer on our servers in the EU          | SaaS, cloud edition                |
| **Notifications**        | Messages about things you follow or that involve you               | inbox, alerts                      |
| **Mark as read**         | Take a notification off the unread ones                            | dismiss, acknowledge (in UI)       |
| **Customize**            | On *Notifications*: opens *Notification settings*. On a workflow or a layout: create a copy for this place that you can change. On a list: choose its columns | subscription settings (in UI) |
| **Notification settings** | The page under *My settings* where you choose, for each kind of event, whether you hear of it in the app, by e-mail, in both or not at all | subscriptions, alert settings (in UI) |
| **Mail server**          | The organization's server Bugaboo sends e-mails through; e-mail is off until an administrator enters it | SMTP relay (in UI) |
| **E-mail and notifications** | The organization settings page where an administrator enters the mail server, sends a test e-mail and switches e-mail on | mail settings, SMTP (in UI) |
| **Test e-mail**          | One e-mail that an administrator sends to themselves (*Send me a test e-mail*) to see that the mail server works | try it (for e-mail) |
| **Link**                 | A web address that you open, in an e-mail or on a screen (to an issue, or to finish creating an account); never the connection between two issues, which is a *Relation* (so *link* stays in the Don't-say column of *Relation*, and TXT-TERMS allows the web sense by entries in `tools/checks/terms-exemptions.toml`) | address (for a link) |
| **Short e-mails**        | An organization choice: e-mails leave out all issue text (title and comments) | — |
| **Handle**               | A person's short unique name for mentions (`@ana.brandt`), changeable in *My settings* | username, nick (in UI) |

## Internal terms (docs and code only, never in the UI)

| Term                | Meaning                                                               |
|---------------------|-----------------------------------------------------------------------|
| Policy engine       | The server component that makes every permission decision             |
| Assignment          | One row of *Who has which role*: a person or group, a role or *No access*, and where it counts. Screens say "Ana has the role Team member in Website" |
| Permission catalogue | The one list of permissions in `bb-policy`, each with its sentence, topic, where it counts and kind |
| Field catalogue     | The one list of built-in fields in `bb-core` that field permissions apply to |
| Readable fields     | The fields one person may read, which the caller passes to the search language (`ReadableFields`); a field outside them is as unknown as one that does not exist. Not the *field catalogue* (the built-in fields of `bb-core`) |
| `PersonRef`         | How every person in an answer is shown, worked out for the viewer: name, role label, *Someone* or *Former user* |
| Readable ID         | The code name (`ReadableId`) of the *Issue ID* people see (`WEB-142`); `IssueId` is the stored key that never changes |
| Edit notice         | The code name (`EditNotice`, `EditNotices`) of the note a person gets about their text: *Ask … to finish*, *Edit anyway*, given back |
| Signed link         | A download link good for 5 minutes and one person; every download checks access again (ADR-0032) |
| Notifications reader | The outbox reader that turns events into notifications for each person (ADR-0033) |
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
