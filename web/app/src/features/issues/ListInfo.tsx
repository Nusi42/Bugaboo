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
}

/** The count line above the list, *302 in Website · 13 shown* (UX §4.2), and what ticks them all. */
export function ListInfo({ scopeName, shown }: ListInfoProps) {
  const { search, ticks, selection } = useListContext();
  const count = useFoundCount(search);
  if (count.data === undefined) {
    return null;
  }
  return (
    <>
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
