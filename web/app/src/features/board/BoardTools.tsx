import { t } from "@bugaboo/messages";
import { Button, buttonLook, Link, part, partOf, selectLook } from "@bugaboo/ui";
import { useState } from "react";

import type { BoardOptions, ListView } from "../../shared/list-view.ts";
import { layoutsAddress } from "../../shared/paths.ts";
import type { KeptIn } from "../../shared/use-list-view.ts";
import { usePermissions } from "../../shared/use-me.ts";
import { useChangeSearch, ViewChoices, type SearchFieldsAnswer } from "../search/index.ts";
import { refreshBoards, type BoardColumn } from "./board-data.ts";
import { CustomizeWindow, type CustomizeChoices } from "./CustomizeWindow.tsx";

export interface BoardToolsProps {
  readonly view: ListView;
  readonly onView: (view: ListView) => void;
  readonly fields: SearchFieldsAnswer;
  /** The columns the board has now. */
  readonly columns: readonly BoardColumn[];
  /** What the options of the board are kept in: only the maker of a saved search has it, so only they see *Customize*. */
  readonly keptIn: KeptIn | undefined;
  readonly choices: CustomizeChoices;
  /** A phone has *Show as* in the sheet of the filters. */
  readonly phone: boolean;
}

/** *Customize*: the window that sets the columns, the rows and the card limits, and the way to the layout *Card*. */
function Customize({
  keptIn,
  view,
  columns,
  choices,
}: Omit<BoardToolsProps, "fields" | "onView" | "phone">) {
  const [open, setOpen] = useState(false);
  const change = useChangeSearch(keptIn?.id ?? "");
  const permissions = usePermissions();
  if (keptIn === undefined) {
    return null;
  }
  const save = async (board: BoardOptions): Promise<void> => {
    await change.mutateAsync({
      text: keptIn.text,
      presentation: { ...keptIn.presentation, board },
      version: keptIn.version,
    });
  };
  return (
    <>
      <Button
        look={buttonLook.quiet}
        onPress={() => {
          setOpen(true);
        }}
      >
        {t("board.customize")}
      </Button>
      {permissions.includes("manage-issue-types") ? (
        <Link href={layoutsAddress(undefined, { form: "card" })}>{t("board.customize-card")}</Link>
      ) : null}
      {open ? (
        <CustomizeWindow
          options={view.board}
          columns={columns}
          choices={choices}
          onSave={save}
          onClose={() => {
            setOpen(false);
          }}
        />
      ) : null}
    </>
  );
}

/**
 * The tools over the board (UX §4.6): *Show as*, *Customize* for the maker of the saved search, and
 * *Refresh*. A board reads again after a change of its own and when the window gets the focus again;
 * *Refresh* is for what others changed meanwhile.
 */
export function BoardTools({ view, onView, fields, phone, ...rest }: BoardToolsProps) {
  return (
    <div {...partOf(part.boardTools)}>
      {phone ? null : (
        <ViewChoices view={view} fields={fields} onView={onView} look={selectLook.inline} />
      )}
      <Customize view={view} {...rest} />
      <Button
        look={buttonLook.quiet}
        onPress={() => {
          void refreshBoards();
        }}
      >
        {t("board.refresh")}
      </Button>
    </div>
  );
}
