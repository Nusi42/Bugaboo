import type { priorityWords, statusGroupWords } from "@bugaboo/api-client";

/** The group a status belongs to: it gives the color and the icon (UX §7.2). */
export type StatusGroup = (typeof statusGroupWords)[number];

/** The priority of an issue, as the server names it. */
export type PriorityWord = (typeof priorityWords)[number];
