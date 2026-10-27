import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { useRouter } from "@tanstack/react-router";
import { Header, ListBox, ListBoxItem, ListBoxSection, Text } from "react-aria-components";

import { closeOverlay } from "./overlay-state.ts";
import { resultByKey, resultKey } from "./quick-search-collect.ts";
import type { QuickFound } from "./use-quick-results.ts";

/** The words of React Aria Components for the slots of an item, not text for people. */
const descriptionSlot = "description";

export interface QuickResultsProps {
  readonly found: QuickFound;
}

/** What quick search found, grouped by where it was found. */
export function QuickResults({ found }: QuickResultsProps) {
  const { groups, waiting } = found;
  const router = useRouter();
  // Opening a result closes the dialog, also when the result is the page that is open already. An
  // action closes it first, so an action that opens a dialog of its own is not closed with it.
  const open = (id: string | number): void => {
    const hit = resultByKey(groups, id);
    if (hit?.run !== undefined) {
      closeOverlay();
      hit.run();
      return;
    }
    if (hit !== undefined) {
      router.history.push(hit.path);
    }
    closeOverlay();
  };
  return (
    <ListBox
      aria-label={t("search.results")}
      {...partOf(part.quickSearchResults)}
      renderEmptyState={() =>
        // Not "nothing found" while the server has yet to answer.
        waiting ? null : <p {...partOf(part.quickSearchEmpty)}>{t("search.nothing")}</p>
      }
      onAction={open}
    >
      {groups.map((group) => (
        <ListBoxSection key={group.id} {...partOf(part.column)}>
          <Header {...partOf(part.quickSearchHeading)}>{group.heading}</Header>
          {group.results.map((result) => (
            <ListBoxItem
              key={resultKey(group, result)}
              id={resultKey(group, result)}
              textValue={result.title}
              {...partOf(part.quickSearchItem)}
            >
              <span {...partOf(part.quickSearchTitle)}>{result.title}</span>
              {result.hint === undefined ? null : (
                <Text slot={descriptionSlot} {...partOf(part.hint)}>
                  {result.hint}
                </Text>
              )}
            </ListBoxItem>
          ))}
        </ListBoxSection>
      ))}
    </ListBox>
  );
}
