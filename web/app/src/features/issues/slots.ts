import type { Schema } from "@bugaboo/api-client";
import type { ReactNode } from "react";

import type { FormView } from "../layouts/index.ts";

/**
 * The slots of the issue page (UX design §4.4): the places where packages other than the issue page
 * itself put their blocks. The page owns the layout (focus view, side panel, phone) and the rules of
 * changing a value; a block only draws its part and calls `onChange`. The signature below does not
 * change without need, and when it must, the old one keeps working (WP-06, M3 README, wave 2).
 *
 * A block is a component that gets {@link IssueBlockProps}. A package adds its block with one call
 * of {@link addIssueBlock} at start-up (like the sidebar sections of the shell) and edits no other
 * file of the issue page.
 */

/** One issue as the person may see it, with what they may do with it (`getIssue`). */
export type IssueDetail = Schema<"IssueDetailBody">;
/** The issue itself: only the fields the person may see are in it. */
export type IssueView = IssueDetail["issue"];

/** The places a block can go. */
export const issueSlots = {
  /** The sub-issues of the issue, below the description (WP-07). */
  subIssues: "sub-issues",
  /** The related issues, below the sub-issues (WP-07). */
  relatedIssues: "related-issues",
  /** *History*: comments and changes, at the end of the main column (WP-07). */
  history: "history",
  /** Sections of own fields that the layout puts into the main column (WP-05). */
  ownFieldSections: "own-field-sections",
  /** The own fields in the fields panel (*Details*), below the built-in ones (WP-05). */
  fieldsPanel: "fields-panel",
} as const;

/** The name of a slot. */
export type IssueSlot = (typeof issueSlots)[keyof typeof issueSlots];

/**
 * What the person may do with the fields of this issue. A field is named by its key: the name of a
 * built-in field (`priority`), or the key of an own field as WP-05 defines it. The page builds it
 * from the answer, so a block never reads the permission rules itself.
 */
export interface IssueFieldMask {
  /** Whether the person sees the field at all (a hidden field is not in the issue). */
  readonly sees: (key: string) => boolean;
  /** Whether the person may change the field in this issue. */
  readonly changes: (key: string) => boolean;
}

/**
 * The values a change sets, by the name of the field as the API knows it. The page adds the
 * version and sends one request; it answers with the new version, or rejects with an `ApiError`
 * (a stale version shows the conflict banner of the page, not of the block).
 */
export type IssueChange = Readonly<Record<string, unknown>>;

/** What every block of the issue page gets. */
export interface IssueBlockProps {
  /** The issue as the person sees it. */
  readonly issue: IssueView;
  /** The address the page was opened with: what a block that reads the issue again asks for. */
  readonly address: string;
  /** The readable ID (`WEB-142`), for links and for the records a block reads. */
  readonly issueKey: string;
  /** The long-text own fields as the server rendered them (WP-05), by the key of the field. */
  readonly ownFieldsHtml: IssueDetail["own_fields_html"];
  /** What the person may change in this issue. */
  readonly mask: IssueFieldMask;
  /**
   * Which fields the layout of the issue page shows to this person, and in which sections (CFG-01).
   * A block draws a field only when `form.shows` says so; the mask still decides what may be changed.
   */
  readonly form: FormView;
  /** The version of the issue now: a block that saves sends the one it read. */
  readonly version: number;
  /** Changes values of the issue against the version; the page shows *Saving…* and *Saved*. */
  readonly onChange: (change: IssueChange) => Promise<void>;
  /** Where the block is drawn: the phone puts the blocks in one column. */
  readonly layout: "wide" | "narrow";
}

/** A block of the issue page: a function component. */
export type IssueBlock = (props: IssueBlockProps) => ReactNode;

/** A block with the slot it goes into and its place among the others in the slot. */
export interface IssueBlockEntry {
  /** Names the block in a test and in the order of the slot. */
  readonly id: string;
  /** Blocks of a slot are drawn in this order, then by `id`. */
  readonly order: number;
  readonly component: IssueBlock;
}

const blocks = new Map<IssueSlot, IssueBlockEntry[]>();

/** Puts a block into a slot. A block with the same `id` in the same slot replaces the old one. */
export function addIssueBlock(slot: IssueSlot, entry: IssueBlockEntry): void {
  const others = (blocks.get(slot) ?? []).filter((known) => known.id !== entry.id);
  blocks.set(
    slot,
    [...others, entry].toSorted((a, b) => a.order - b.order || a.id.localeCompare(b.id)),
  );
}

/** The blocks of a slot, in the order they are drawn. */
export function issueBlocks(slot: IssueSlot): readonly IssueBlockEntry[] {
  return blocks.get(slot) ?? [];
}

/**
 * The mask of an issue answer: `changeable` names the fields the person may change, built-in or
 * own. An own field is seen when it has a value or the person may change it (a hidden field is in
 * neither).
 */
export function maskOf(detail: Readonly<IssueDetail>): IssueFieldMask {
  const changeable = new Set<string>(detail.changeable);
  const own = detail.issue.own_fields ?? {};
  return {
    sees: (key) => key in detail.issue || key in own || changeable.has(key),
    changes: (key) => detail.may_change && changeable.has(key),
  };
}
