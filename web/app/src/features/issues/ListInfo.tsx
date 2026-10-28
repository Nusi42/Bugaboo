import { t } from "@bugaboo/messages";
import { Announce, part, partOf } from "@bugaboo/ui";

import { useFoundCount } from "./found-issues.ts";
import { useListContext } from "./list-context.ts";
import { TickAll } from "./TickAll.tsx";

export interface ListInfoProps {
  /** The name of the project, for the count line; none for every project. */
  readonly scopeName: string | undefined;
  /** The issues the list draws now. */
  readonly shown: readonly string[];
  /** The text of the search has a mistake: the list below is the last one without. */
  readonly stale: boolean;
}

/** The count line above the list, *302 in Website · 13 shown* (UX §4.2), and what ticks them all. */
export function ListInfo({ scopeName, shown, stale }: ListInfoProps) {
  const { search, ticks, selection, failure } = useListContext();
  const count = useFoundCount(search);
  // With no earlier list (the address itself has the mistake) the error says it all.
  const note = stale && failure === null;
  if (count.data === undefined) {
    return null;
  }
  return (
    <>
      {note ? <output {...partOf(part.text)}>{t("search.found-stale")}</output> : null}
      <p {...partOf(part.muted)}>
        {t("issues.count-line", {
          total: count.data,
          scope: scopeName ?? t("issues.everywhere"),
          shown: shown.length,
        })}
      </p>
      <Announce>{t("issues.count", { count: count.data })}</Announce>
      {ticks ? (
        <TickAll search={search} shown={shown} total={count.data} selection={selection} />
      ) : null}
    </>
  );
}
