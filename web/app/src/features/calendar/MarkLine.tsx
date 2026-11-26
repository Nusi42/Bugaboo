import { t } from "@bugaboo/messages";
import { IssueId, Link, part, partOf } from "@bugaboo/ui";

import type { Mark } from "./marks.ts";

/**
 * A goal on its *Due date* or a release on its *Release date* (M10-Q8): a diamond and the words that
 * say which it is, never the diamond alone (R5). It is not dragged on the calendar; it opens the goal,
 * or the page *Releases*.
 */
export function MarkLine({ mark }: { readonly mark: Mark }) {
  return (
    <li {...partOf(part.releaseMark, { variants: [mark.kind] })}>
      <span {...partOf(part.releaseMarkDiamond)} aria-hidden />
      <Link
        href={mark.href}
        label={
          mark.kind === "goal"
            ? t("calendar.goal-due", { key: mark.id ?? "", title: mark.name ?? "" })
            : t("calendar.release-on", { name: mark.name ?? "" })
        }
      >
        <span {...partOf(part.releaseMarkLabel)}>
          {mark.id === undefined ? null : <IssueId id={mark.id} />}
          {mark.id === undefined ? null : " "}
          <span {...partOf(part.calendarItemTitle)}>
            {mark.id === undefined
              ? t("calendar.release-mark", { name: mark.name ?? "" })
              : (mark.name ?? "")}
          </span>
        </span>
      </Link>
    </li>
  );
}
