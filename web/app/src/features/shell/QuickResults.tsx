import { t } from "@bugaboo/messages";
import { cx } from "@bugaboo/ui";
import { Header, ListBox, ListBoxItem, ListBoxSection, Text } from "react-aria-components";

import { closeOverlay } from "./overlay-state.ts";
import styles from "./QuickSearch.module.css";
import type { ResultGroup } from "./use-quick-results.ts";

/** The words of React Aria Components for the slots of an item, not text for people. */
const descriptionSlot = "description";

export interface QuickResultsProps {
  readonly groups: readonly ResultGroup[];
}

/** What quick search found, grouped by where it was found. */
export function QuickResults({ groups }: QuickResultsProps) {
  return (
    <ListBox
      aria-label={t("search.results")}
      className={cx(styles.results)}
      renderEmptyState={() => <p className={styles.empty}>{t("search.nothing")}</p>}
      onAction={closeOverlay}
    >
      {groups.map((group) => (
        <ListBoxSection key={group.heading} className={cx(styles.group)}>
          <Header className={styles.heading}>{group.heading}</Header>
          {group.results.map((result) => (
            <ListBoxItem
              key={result.id}
              id={result.id}
              textValue={result.title}
              href={result.path}
              className={cx(styles.item)}
            >
              <span className={styles.title}>{result.title}</span>
              {result.hint === undefined ? null : (
                <Text slot={descriptionSlot} className={styles.hint}>
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
