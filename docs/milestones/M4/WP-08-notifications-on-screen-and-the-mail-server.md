# WP-08 — Notifications on screen, notification settings, the mail server

**Wave 3 · Folder / branch:** `bugaboo.win4` / `WIN4`
**Features:** NTF-01, NTF-02 (screens), PRIV-08 (switching e-mail on); UX design §2 (menu, phone bar), §4.1 (start page), §4.11, §5.1 (*Connections › E-mail & notifications*), the first start (§4.13); ADR-0033; follow-up M1-F38 (screens).

## Goal

People see their notifications and choose what they hear of. An administrator connects the mail server and tries it.

## Owns

- `web/app/src/features/notifications/**` (new): the page, *Customize*, the start-page block (it replaces `features/mentions/MentionsBlock.tsx`).
- The menu entry with its count, and the phone's bottom bar entry.
- *Organization settings › Connections › E-mail & notifications*, and the optional e-mail step of the first start.
- The handle in *My settings*.
- The *Confirm your e-mail address* screens of signing up.
- The notification parts in `design/theme.kdl`.
- `userDocs/notifications/*` (new).

## Tasks

1. **The page (UX §4.11):**
   - grouped by issue, newest first;
   - *Open* goes to the exact comment or change; *Mark as read*;
   - *All*, *Unread* and *Mentions*;
   - route `/notifications` and shortcut `g n`.
2. **The menu and the phone:** *Notifications 3* in the menu; the bottom bar *Home · Search · New · Notifications · More*.
3. **The start page:** the newest unread notifications instead of the *Mentions* block (M4-Q7).
4. ***Customize*:** one row for each kind of event, *In the app* and *By e-mail*, with the defaults of M4-Q5.
5. **The mail server:**
   - host, port, security, sender and password (shown only as *set*);
   - *Send me a test e-mail*, with the server's answer in plain words when it fails;
   - *Short e-mails*;
   - e-mail stays off until switched on;
   - the same step, optional, in the first start.
6. **Handle:** shown and changed in *My settings*, refused in words when it is taken.
7. **Signing up when mail is on:** the *"We sent a link to …"* page, the link's page, and a link that ran out (with *Send a new link*).
8. **Screenshots** in four themes, computer and phone. The builder checks them against UX §1.1 before pushing.

## Acceptance tests (write first)

- Playwright, two browsers:
  - Ben comments on an issue Ana follows;
  - Ana's count goes up and *Open* lands on the comment;
  - after *Mark as read* the count goes down.
- Playwright: *Customize* turns off e-mail for follows, and the fake mailer receives nothing for the next comment.
- Playwright: an administrator enters the mail server, *Send me a test e-mail* reaches the fake mail server; a wrong password is refused in words.
- Playwright: a Guest's notification of an issue shows *Someone* where *Who sees people* says so.
- Four themes, computer and phone, axe.

## Checks

None new.

## Depends on

WP-06 (the notification routes, the mail settings, handles, the confirmation link).
