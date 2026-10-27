import { t } from "@bugaboo/messages";
import { Button, part, partOf } from "@bugaboo/ui";

import { useListContext } from "./list-context.ts";
import { NoIssues } from "./NoIssues.tsx";

/**
 * A list with nothing in it says so in one sentence and offers the one thing to do (UX §6.1): a
 * project without issues offers the first one, a search that finds none offers to take its filters
 * away.
 */
export function ListEmpty() {
  const { search, project, onClear } = useListContext();
  if (search.text === "" && project !== undefined) {
    return <NoIssues project={project} />;
  }
  return (
    <div {...partOf(part.stack)}>
      <p {...partOf(part.text)}>{t("search.found-none")}</p>
      {search.text === "" ? null : <Button onPress={onClear}>{t("issues.remove-filters")}</Button>}
    </div>
  );
}
