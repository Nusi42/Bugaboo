import { t } from "@bugaboo/messages";
import { SearchField, useLayout, partOf, part } from "@bugaboo/ui";
import { useId, useState, type ReactNode } from "react";
import { Button } from "react-aria-components";

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

interface RowsProps extends Pick<ListEditorProps, "rows" | "openKey" | "onOpen"> {
  /** The name of the list. */
  readonly label: string;
}

/** The rows: the whole row opens its item (R8), the name first and the rest of the row after it. */
function Rows({ rows, label, openKey, onOpen }: RowsProps) {
  const ids = useId();
  const nameId = (key: string): string => [ids, "name", key].join("-");
  const detailsId = (key: string): string => [ids, "details", key].join("-");
  return (
    <ul {...partOf(part.list, { variants: ["snug"] })} aria-label={label}>
      {rows.map((row) => (
        <li key={row.key}>
          <Button
            {...partOf(part.listEditorOpen, { states: [row.key === openKey && "open"] })}
            aria-expanded={row.key === openKey}
            // The name of the button is the name of the item; the rest of the row describes it.
            aria-labelledby={nameId(row.key)}
            aria-describedby={detailsId(row.key)}
            onPress={() => {
              onOpen(row.key);
            }}
          >
            <span id={nameId(row.key)} {...partOf(part.listEditorName)}>
              {row.name}
            </span>
            <span id={detailsId(row.key)} {...partOf(part.listEditorDetails)}>
              {row.details}
            </span>
          </Button>
        </li>
      ))}
    </ul>
  );
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
  // A phone shows one thing at a time: the open item takes the screen, and Back closes it. The
  // list stays in the page, hidden, so the focus can go back to the row that was opened.
  const alone = useLayout() === "phone" && open !== undefined;
  return (
    <div {...partOf(part.listEditor)}>
      <div {...partOf(part.listEditorList, { states: [alone && "hidden"] })} hidden={alone}>
        <SearchField
          label={searchLabel}
          clearLabel={t("search.clear")}
          value={query}
          onChange={setQuery}
        />
        {shown.length === 0 ? <p {...partOf(part.listEditorNothing)}>{nothing}</p> : null}
        <Rows rows={shown} label={searchLabel} openKey={openKey} onOpen={onOpen} />
      </div>
      {open === undefined ? null : (
        <SidePanel title={open.name} onClose={onClose}>
          {open.editor}
        </SidePanel>
      )}
    </div>
  );
}
