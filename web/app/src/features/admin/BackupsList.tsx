import { t } from "@bugaboo/messages";
import { partOf, part } from "@bugaboo/ui";
import type { Selection } from "react-aria-components";
import { Collection, ListBox, ListBoxItem, ListLayout, Virtualizer } from "react-aria-components";

import { formatDateTime } from "../../shared/format.ts";
import { formatSize } from "../../shared/file-size.ts";
import { makerText, type BackupRow } from "./backups.ts";

/** The word of React Aria Components for a list box where one row is chosen. */
const ONE_CHOSEN = "single";

export interface BackupsListProps {
  readonly rows: readonly Readonly<BackupRow>[];
  /** The name of the backup the restore command is shown for. */
  readonly chosen: string | undefined;
  readonly onChoose: (name: string) => void;
  /** The list could not be read: it is not empty, so it does not say that there are no backups. */
  readonly failed: boolean;
}

/** A row with the key the list chooses it by: its name. */
interface ChoosableRow extends BackupRow {
  readonly id: string;
}

function choosable(row: Readonly<BackupRow>): ChoosableRow {
  return { ...row, id: row.name };
}

/** The name of the one backup a selection holds, if it holds one. */
function nameOf(selection: Readonly<Selection>): string | undefined {
  if (selection === "all") {
    return undefined;
  }
  const [first] = selection;
  return typeof first === "string" ? first : undefined;
}

/**
 * The backups, newest first, in a box of its own height that draws only the rows that are in it; one is chosen, and
 * the command that restores it stands under the list.
 */
export function BackupsList({ rows, chosen, onChoose, failed }: Readonly<BackupsListProps>) {
  return (
    <Virtualizer layout={ListLayout}>
      <ListBox
        aria-label={t("backup-page.list")}
        selectionMode={ONE_CHOSEN}
        disallowEmptySelection
        selectedKeys={chosen === undefined ? [] : [chosen]}
        onSelectionChange={(selection: Readonly<Selection>) => {
          const name = nameOf(selection);
          if (name !== undefined) {
            onChoose(name);
          }
        }}
        renderEmptyState={() =>
          failed ? null : <p {...partOf(part.pickListFacts)}>{t("backup-page.none")}</p>
        }
        {...partOf(part.pickList)}
      >
        <Collection items={rows.map((row) => choosable(row))}>
          {(row) => (
            <ListBoxItem
              id={row.id}
              textValue={row.name}
              {...partOf(part.pickListRow, { variants: ["choose"] })}
            >
              <span {...partOf(part.pickListName)}>{row.name}</span>
              <span {...partOf(part.pickListFacts)}>{formatDateTime(row.moment)}</span>
              <span {...partOf(part.pickListFacts)}>
                {t("backup-page.row-facts", {
                  size: formatSize(row.size),
                  maker: makerText(row),
                })}
              </span>
            </ListBoxItem>
          )}
        </Collection>
      </ListBox>
    </Virtualizer>
  );
}
