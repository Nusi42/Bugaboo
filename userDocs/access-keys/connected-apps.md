# Connected apps

An **AI app** such as Claude Code or Claude Desktop can work with your account. It always works as you and never does more than you can. You decide, once for each app, what it may do.

This page is for AI apps that you sign in to Bugaboo from yourself. A script that cannot ask you uses an [access key](README.md) instead.

## Allowing an app

1. In the app, add Bugaboo as a server (the address is the one you open Bugaboo at, followed by `/mcp`).
2. The app opens a page of Bugaboo: "Claude Code wants to work with your account". If you are not signed in, you sign in first and come back to this page.
3. Look at what it asks:
   - **See only**: the app can look at what you can look at and change nothing. An app that asks for this only is shown the sentence and nothing to choose.
   - **See and change**: the app can also change what you may change, but it cannot manage people, roles or settings. This is offered only when the app asks for it, and you can always allow less than it asked. Both choices are explained on the page.
4. If you want, tick the **projects** the app is for. It then works in those projects and in the projects inside them, and nowhere else. Tick none and it works in every project you can see.
5. Press **Allow**. The app gets what it needs and you can go back to it. Press **Cancel** and the app is sent away with nothing.

The name on the page is the one the app gives itself. Bugaboo cannot check it, so allow only apps you started yourself. The page also says where the app runs: an app on your own computer says so, and an app on the web shows the web address it comes back to, which an administrator allowed.

## What an app can and cannot do

- The app does at most what you can do, and at most what you allowed. If an administrator takes a role away from you, the app loses it at its next call.
- Everything the app changes says so in the history: "Ada Brandt via Claude Code". What it changes in one session is one run that you can undo in one step.
- An app that was allowed for a while without being used runs out after 30 days, and no app works longer than the longest time your administrators set. Then it asks you again.
- The app never reaches your sign-in: not your password, not your second step, not your places where you are signed in, not your keys.
- Administrators can switch AI apps off for the organization. The apps you allowed wait until they are switched on again.
- An app that makes too many calls in a minute has to wait a little. Nothing it asked for during that time is done.

## Connected apps and Take back

*My settings*, under "Connected apps", lists the apps you allowed in a table: each with its name, what it may do, which projects it is for, when you allowed it and when it was last used.

Press **Take back** next to an app to end what it may do. It stops working at once and has to ask you again. What it already did stays as it is; if you want it undone, undo the run of that session.

"Sign out everywhere" takes back every app as well, and so it does for your access keys.
