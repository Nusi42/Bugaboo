# The relation map

The **relation map** is a picture of everything connected to an issue: in any direction, through any number of steps. It answers questions like "what is all this waiting on?" and "what else is late if this slips?".

## Opening it

- On an issue, press **Show relation map** under *Related issues*, or choose it from *More*. The address is `/issues/WEB-142/map`, so you can share it.
- For a search, choose **Show as › Map**. The map then draws the results of the search, as many as one map holds, and the relations between them. In a project, the map holds the issues of the project and of the projects inside it.

## How it is laid out

- The issue you start from stands on the **left**, with an accent border and a small *Start* tab.
- To its right stands one **column for each step away**: first the issues one relation away, then the issues two relations away, and so on. Each column is named: *1 step away*, *2 steps away*.
- Inside a column the issues always stand in the same order: by kind of relation, then by Issue ID. The same answer always gives the same picture, and nothing moves by itself. The picture changes only when you change *Steps away*, *Kinds shown* or the start.
- Each issue is a card with its status icon, ID, type and title, tinted by its status group, with a colored strip at the left for a high or critical priority, like a line of a list.
- On the map of a search, each group of connected results starts with its first result in the order of the search.

## The lines

Each kind of relation has its own line and its own words, so a color is never the only sign:

- **blocks** is orange with an arrow: the arrow points to the issue that waits. Every kind that sets a date effect (*Must finish first*) is drawn like this.
- **part of** (sub-issues) is violet: the arrow points to the parent.
- **related to** is grey without an arrow, because it has no direction. Every kind that reads alike from both sides is drawn like this.
- The other kinds take a dashed grey line and a red line in the order of *Organization settings › Kinds of relation*: at the start, **duplicate of** is dashed grey and **causes** is red. Their arrow points to the issue that reads the second name (*has duplicate*, *is caused by*).

The words of each line stand on its middle. *What the lines mean* beside the map says the same, with the colors of the cards.

## The tools

- **Steps away**: *1*, *2*, *3* or *Everything*. *Everything* walks as many steps as a search may (`within N steps`).
- **Kinds shown**: a chip for each kind of relation and one for *part of*. Turn a kind off to leave its relations out of the map; the issues reached only through it go too.
- **Picture · List**: the picture, or the same chains as sentences.
- **Zoom** with a percentage, and **Fit**, which makes the whole map fit its box. Drag the background with the mouse to move around.
- *Only issues you may see are shown.* stands in the tools as a reminder.

Your choices of *Steps away* and *Kinds shown* are in the address, and *Start from here* keeps them.

## Following a chain

- **Point at** an issue (or move to it with the keyboard) to see its chain back to the start; everything else fades for the moment. Nothing moves.
- **Click** an issue to keep its chain. The side shows it as numbered sentences, for example "WEB-142 blocks WEB-147, which is blocked by WEB-148", with **Start from here** and **Open issue**.
- **Start from here** makes that issue the start of a new map, so you can walk along a chain. *Back* in the browser goes back to the map before.

## Loops

A loop in *blocks* (WEB-160 blocks WEB-161, which blocks WEB-160) can only be left from before loops were refused. The map names it with a warning: "Loop found: WEB-160 blocks WEB-161, which blocks WEB-160. Neither can start until one of these relations is removed." The timeline shows the same loop as the plan warning *Loop*.

## The list form

The list form holds the same chains as the picture, grouped by steps away: each issue with its status icon and the colored strip of its priority, after the sentence of its chain up to it. A click on a row opens the issue. A phone shows the list first, with **Show as picture**; for people who use a reader, the picture is one image, named with the number of issues it holds, and the list is its reading order.

## What you see

- Only issues you may see are on the map. A chain never passes through an issue you may not see, so the map tells nothing about one: no card, no line and no gap.
- A map holds a limited number of issues. When more are connected, the issues nearer the start are kept, and the map says so and suggests fewer steps away or fewer kinds. For a search it says that more issues are found than the map shows.

The most issues one map holds and the most steps it walks are in [Limits of a search](search-limits.md).

See also [Sub-issues and relations](relations.md).
