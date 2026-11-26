import { t } from "@bugaboo/messages";
import { ListCell, part, partOf, type ListColumn } from "@bugaboo/ui";
import { GridListItem } from "react-aria-components";

import { cellOf, listWord } from "../../shared/list-columns.ts";
import { groupName, groupsOfKeys, type GroupOfKey } from "./group-keys.ts";
import { columnTotal, sumsOf, useGroupTotals } from "./group-totals.ts";
import { useListContext } from "./list-context.ts";

const selectColumn = "select" satisfies ListColumn;

/** The words of each column of a row of totals: *Total* and the count first, then the sum of each column that has one. */
function totalWords(
  word: string,
  count: string,
  sum: (word: string) => string | undefined,
): string | undefined {
  if (word === listWord.id) {
    return t("issues.totals");
  }
  return word === listWord.title ? count : sum(word);
}

/**
 * The quiet row of totals under a group of the list (QRY-08, M10-Q15): the count of the group and, under each column of
 * a length or an own number, its sum, from the one read of the totals of the list; a sum over fewer issues than the
 * group holds says over how many. It is a row of the list like the head of a group, reached by the arrow keys and
 * never ticked. On a phone it is one line of the sums with their names.
 */
export function GroupTotals({ group }: { readonly group: GroupOfKey }) {
  const context = useListContext();
  const { words, own, groupBy, search } = context;
  const totals = useGroupTotals(search, groupBy, sumsOf(words, own));
  const answer = totals.data;
  const by = groupBy ?? undefined;
  const at =
    answer === undefined
      ? -1
      : groupsOfKeys(answer.groups, by === undefined ? undefined : own[by], by).findIndex(
          (each) => each.id === group.id,
        );
  const row = answer?.groups[at];
  if (answer === undefined || row === undefined) {
    return null;
  }
  const count = t("issues.totals-count", { count: row.issues });
  const sum = (word: string): string | undefined => columnTotal(word, row, answer, own);
  const itemId = `${group.id}/totals`;
  const spoken = [
    t("issues.totals-of", { group: groupName(group) }),
    count,
    ...words.flatMap((word) => sum(word) ?? []),
  ].join(t("issues.totals-separator"));
  return (
    <GridListItem
      id={itemId}
      textValue={spoken}
      aria-label={spoken}
      isDisabled
      {...partOf(part.listRow, {
        variants: [context.phone ? "phone" : "grid"],
        states: ["totals"],
      })}
    >
      {context.ticks && !context.phone ? <ListCell column={selectColumn}>{null}</ListCell> : null}
      {context.phone ? (
        <ListCell column={cellOf(listWord.title)}>{spoken}</ListCell>
      ) : (
        words.map((word) => (
          <ListCell key={word} column={cellOf(word)}>
            {totalWords(word, count, sum)}
          </ListCell>
        ))
      )}
    </GridListItem>
  );
}
