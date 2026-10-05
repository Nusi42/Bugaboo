import { t } from "@bugaboo/messages";
import { cx, SearchField } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";
import { Button } from "react-aria-components";

import styles from "./ListEditor.module.css";
import { SidePanel } from "./SidePanel.tsx";

/** One row of the list. */
export interface ListRow {
  /** A name that does not change, for the address and for React. */
  readonly key: string;
  /** What the person calls the item: the list is searched by it, and the panel is named after it. */
  readonly name: string;
  /** The rest of the row, after the name. */
  readonly details?: ReactNode;
  /** What the side panel shows when this item is open: the editor of a simple item. */
  readonly editor: ReactNode;
}

export interface ListEditorProps {
  readonly rows: readonly ListRow[];
  /** The name of the search box. */
  readonly searchLabel: string;
  /** What to say when nothing is left after searching. */
  readonly nothing: string;
  /** The key of the item that is open, from the address. */
  readonly openKey: string | undefined;
  readonly onOpen: (key: string) => void;
  readonly onClose: () => void;
}

/**
 * The list → editor pattern of every settings screen (UX §5.1): a searchable list, where a simple
 * item opens in a side panel next to it. (A big item gets a page of its own: its row is a link.)
 * Which item is open lives in the address, so it can be shared and the back button closes it.
 */
export function ListEditor({
  rows,
  searchLabel,
  nothing,
  openKey,
  onOpen,
  onClose,
}: ListEditorProps) {
  const [query, setQuery] = useState("");
  const shown = rows.filter((row) => row.name.toLowerCase().includes(query.trim().toLowerCase()));
  const open = rows.find((row) => row.key === openKey);
  return (
    <div className={styles.frame}>
      <div className={styles.list}>
        <SearchField
          label={searchLabel}
          clearLabel={t("search.clear")}
          value={query}
          onChange={setQuery}
        />
        {shown.length === 0 ? <p className={styles.nothing}>{nothing}</p> : null}
        <ul className={styles.rows} aria-label={searchLabel}>
          {shown.map((row) => (
            <li key={row.key} className={styles.row}>
              <Button
                className={cx(styles.open)}
                aria-expanded={row.key === openKey}
                onPress={() => {
                  onOpen(row.key);
                }}
              >
                {row.name}
              </Button>
              {row.details}
            </li>
          ))}
        </ul>
      </div>
      {open === undefined ? null : (
        <SidePanel title={open.name} onClose={onClose}>
          {open.editor}
        </SidePanel>
      )}
    </div>
  );
}
