import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import { Header, ListBox, ListBoxItem, ListBoxSection, Text } from "react-aria-components";

import { busyKeys, resultKey } from "./quick-search-collect.ts";
import type { QuickFound } from "./use-quick-results.ts";

/** The words of React Aria Components for the slots of an item, not text for people. */
const descriptionSlot = "description";

export interface QuickResultsProps {
  readonly found: QuickFound;
  /** Opens a result by its name. */
  readonly onOpen: (key: string | number) => void;
}

/** What quick search found, grouped by where it was found. */
export function QuickResults({ found, onOpen }: QuickResultsProps) {
  const { groups, waiting } = found;
  return (
    // The list is busy while the server has yet to answer for the words now typed.
    <div aria-busy={waiting}>
      <ListBox
        disabledKeys={busyKeys(groups)}
        aria-label={t("search.results")}
        {...partOf(part.quickSearchResults)}
        renderEmptyState={() =>
          // Not "nothing found" while the server has yet to answer.
          waiting ? null : <p {...partOf(part.quickSearchEmpty)}>{t("search.nothing")}</p>
        }
        onAction={onOpen}
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
    </div>
  );
}
