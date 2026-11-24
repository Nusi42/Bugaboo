# About

Open *My settings*, then "About". Everybody who is signed in can open it.

## The version and how long it gets fixes

"Version" is the version of Bugaboo that runs on your server, and "Released" is the day that version came out.

"Support period" says how long this release gets security fixes:

- **Only the newest release gets fixes.** A version before 1.0 is a preview. When the next release comes out, fixes go only into that one, so update to it.
- **Security fixes until …** From 1.0 on, a release gets security fixes until the day shown. Update to a newer release before that day. [Update Bugaboo](../install/upgrade.md).

## The license

Bugaboo is free software under the AGPL-3.0-or-later license. The page names it under "License".

## The parts inside

"Parts inside Bugaboo" lists every part inside this release, with its version and its license. The list shows a page of parts at a time and stays quick however many parts there are; press "Show more parts" to read on.

Above the list you can download:

- **The parts list (CycloneDX)** and **the parts list (SPDX)**: the same list as a file that other tools read. They are the two usual formats for a list of parts. The CycloneDX file also says, for each known weakness of a part, why it does not affect Bugaboo.
- **Third-party notices**: the license texts and notices of the parts, as one text file.

The files belong to the release that runs. The CycloneDX file and the notices are also next to every release on the release page, where you can check them like the program: [Check a download](../install/verify-a-download.md).

If a build carries no list of its parts (a build that was not created for a release), the page says so and offers no downloads.
