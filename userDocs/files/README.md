# Files

You can add files to an issue and to a comment: screenshots, logs, documents. Files are in the **Files** block of the issue page, below the related issues. The files of a comment are under the text of that comment.

## Adding files

- **Drop** files on the place at the top of **Files**.
- **Paste** a screenshot with Ctrl+V (Cmd+V on a Mac) while the issue page is open. A screenshot that you paste into the box of a comment goes to that comment, not to the issue.
- Press **Upload** and choose files on your device.

Under a comment you press **Add files**. For a comment you are still writing, the files you chose wait on your device, in a list with **Take out** next to each one, and they are sent as soon as you post the comment.

Each file shows a bar while it is sent, under the files that are there. Big files are sent in parts: if your connection is lost, Bugaboo goes on from where it stopped instead of starting again. If it cannot go on, the row says that the connection was lost and which file was not sent; **Try again** goes on with what the server has, and **Close** throws it away.

## What is refused, and why you are told

A file is refused in words, at once:

- **Too big:** the row says the limit, for example *Too big: at most 25 MB per file*. The limit is 25 MB unless your administrator or the lead of the project changed it. It is never more than 100 MB.
- **A type that is not accepted:** the organization can list types it never accepts, such as *exe* or *bat*. Bugaboo looks at what is in the file, not only at its name, so renaming a file does not get it past. Dots and spaces at the end of a name are dropped (`run.bat.` is `run.bat`), and a name with an invisible character in it is refused.
- **Empty:** a file with nothing in it is not added.
- **No right:** you need the right to change the issue, or to comment, to add files. If you lose that right while a file is on its way, it stops.
- **Too many at once:** you can have 20 files on their way at a time, and an issue holds 200 files, the ones in the trash included until they are removed for good.
- **No room on the server:** when the disk of the server is nearly full, the row says so; try again later or tell your administrator.

## Looking at a file

A picture or the first page of a PDF has a small preview, which shows a moment after the file is added: Bugaboo makes it in the background and the picture appears by itself. Press it to see it larger. Other files show the sign of their type. Each row says the size, who added the file and when.

**Download** saves the file. The download address works for five minutes and only for you, and Bugaboo checks again that you may see the file every time you use it, so an address you pass on does not work for somebody else, and does not work after you lose access.

Files are never shown inside the page and are never run: a download is always saved as a file.

## Deleting a file

**Delete** takes a file off the issue. **Undo** brings it back right away. A deleted file is kept for 30 days and then removed for good. You can delete a file that you added while you may still add files there, and anybody who may change issues can delete any file.

## Limits for the people who look after Bugaboo

- **Biggest file** (*Organization settings*): the most megabytes one file may have, from 1 to 100. A project lead can set a lower number for a project, never a higher one.
- **Types of file that are never accepted** (*Organization settings*): a list separated by commas, like `exe, bat`.
- An issue holds at most 200 files.
- A person can have 20 uploads open, and together they may declare at most four times the biggest file.
- A part of a file is refused when the disk of the server would be left with less than 1 GB free.
- Files are stored once by their content: the same file added twice takes the room of one.
- Unfinished uploads are cleaned up after one day.

Adding a file, deleting it and bringing it back with **Undo** are rows in the **History** of the issue, each with the name of the file. A file on a comment belongs to its comment and has no row of its own.
