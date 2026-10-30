import { t } from "@bugaboo/messages";
import { Alert, alertTone, Announce, Button, buttonLook, part, partOf } from "@bugaboo/ui";

import { showSearchSheet } from "../../shared/search-sheet.ts";
import { countLine } from "./count-line.ts";
import { useFoundCount } from "./found-issues.ts";
import { useListContext } from "./list-context.ts";

export interface ListInfoProps {
  /** The name of the project, for the count line; none for every project. */
  readonly scopeName: string | undefined;
  /** The issues the list draws now. */
  readonly shown: readonly string[];
  /** The text of the search has a mistake: the list below is the last one without. */
  readonly stale: boolean;
  /** How many columns the person chose; those that do not fit give way and are counted here. */
  readonly chosen: number;
}

/**
 * The count line above the list, *302 in Website · 13 shown* (UX §4.2), and the line that says how
 * many chosen columns gave way for lack of room (a phone draws its own few and says nothing).
 */
export function ListInfo({ scopeName, shown, stale, chosen }: ListInfoProps) {
  const { search, failure, phone, words } = useListContext();
  const hidden = phone ? 0 : chosen - words.length;
  const count = useFoundCount(search);
  // With no earlier list (the address itself has the mistake) the error says it all.
  const note = stale && failure === null;
  if (count.data === undefined) {
    return null;
  }
  const scope = scopeName ?? t("issues.everywhere");
  return (
    <>
      {note ? (
        <Alert tone={alertTone.info}>
          {t("search.found-stale")}
          {phone ? (
            <div>
              <Button look={buttonLook.quiet} onPress={showSearchSheet}>
                {t("search.show-mistake")}
              </Button>
            </div>
          ) : null}
        </Alert>
      ) : null}
      <p {...partOf(part.muted)}>{countLine(count.data, scope, shown.length)}</p>
      {hidden > 0 ? (
        <p {...partOf(part.muted)}>{t("issues.columns-hidden", { count: hidden })}</p>
      ) : null}
      <Announce>{t("issues.count", { count: count.data })}</Announce>
    </>
  );
}
