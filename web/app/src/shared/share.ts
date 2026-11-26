import type { Schema } from "@bugaboo/api-client";

/** The words of the two kinds a saved search or a dashboard is shared with. */
export const shareKind = { group: "group", project: "project" } as const satisfies Record<
  string,
  Schema<"ShareKind">
>;

/** What a share is about: a group or a project. */
export interface ShareTarget {
  readonly kind: Schema<"ShareKind">;
  readonly id: string;
}
