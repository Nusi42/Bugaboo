import { partOf } from "./parts.ts";
import { part } from "./tokens.ts";

/**
 * The ID of an issue (`WEB-142`), the one way it is drawn: in the code font wherever it stands, in a list, on the issue
 * page, in a notification, in *Quick search*, in a picker and in a sentence (UX §7.3, M5-F137). It keeps the color of
 * what it stands in.
 */
export function IssueId({ id }: { readonly id: string }) {
  return <span {...partOf(part.issueId)}>{id}</span>;
}
