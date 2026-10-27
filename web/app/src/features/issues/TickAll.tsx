import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, part, partOf } from "@bugaboo/ui";
import { useMutation } from "@tanstack/react-query";

import { messageOf } from "../../shared/errors.ts";
import { readIssueIds, type FoundSearch } from "./found-issues.ts";
import { MAX_TICKED, tickAllOffer } from "./tick-all.ts";
import type { IssueSelection } from "./use-selection.ts";

export interface TickAllProps {
  readonly search: FoundSearch;
  /** The issues the list draws now. */
  readonly shown: readonly string[];
  /** How many issues the search finds. */
  readonly total: number;
  readonly selection: IssueSelection;
}

/**
 * Under the head of the list, once every row shown is ticked and more issues match: *Tick all 302
 * results* (UX §4.2), up to the most one change may name; above that the sentence that asks to narrow
 * the search (M5-Q9).
 */
export function TickAll({ search, shown, total, selection }: TickAllProps) {
  const ticking = useMutation({
    mutationFn: () => readIssueIds(search, MAX_TICKED),
    onSuccess: selection.replace,
  });
  const allShownTicked = shown.length > 0 && shown.every((id) => selection.has(id));
  const offer = tickAllOffer({ shown: shown.length, total });
  if (!allShownTicked || offer.kind === "none") {
    return null;
  }
  return (
    <div {...partOf(part.row)}>
      {offer.kind === "all" ? (
        <Button
          isPending={ticking.isPending}
          onPress={() => {
            ticking.mutate();
          }}
        >
          {t("issues.tick-all", { count: offer.count })}
        </Button>
      ) : (
        <p {...partOf(part.muted)}>{t("issues.tick-narrow", { limit: offer.limit })}</p>
      )}
      {ticking.isError ? <Alert tone={alertTone.danger}>{messageOf(ticking.error)}</Alert> : null}
    </div>
  );
}
