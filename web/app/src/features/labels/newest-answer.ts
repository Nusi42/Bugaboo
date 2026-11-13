import type { Schema } from "@bugaboo/api-client";

/** The tags and labels of an issue as a read or the answer of a change says them. */
type Answer = Schema<"IssueLabelsAndTagsBody">;

/** Whether what the cache keeps for the tags and labels of an issue is such an answer. */
function isAnswer(known: unknown): known is Answer {
  return typeof known === "object" && known !== null && "version" in known && "issue" in known;
}

/**
 * What the row of tags and labels shows after the answer of a change: the answer, unless what it
 * shows is of a newer version of the issue already (the answer of a change made after came first).
 */
export function newestAnswer(known: unknown, made: Answer): Answer {
  return isAnswer(known) && known.version > made.version ? known : made;
}
