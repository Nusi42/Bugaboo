import { t } from "@bugaboo/messages";
import { Button, buttonLook } from "@bugaboo/ui";
import { useState } from "react";

import type { BoardOptions, ListView } from "../../shared/list-view.ts";
import type { KeptIn } from "../../shared/use-list-view.ts";
import { useChangeSearch } from "../search/index.ts";
import type { BoardColumn } from "./board-data.ts";
import { CustomizeCardButton } from "./CustomizeCardButton.tsx";
import { CustomizeWindow, type CustomizeChoices } from "./CustomizeWindow.tsx";

export interface CustomizeButtonProps {
  readonly view: ListView;
  /** The columns the board has now. */
  readonly columns: readonly BoardColumn[];
  /** What the options of the board are kept in: only the maker of a saved search has it, so only they see *Customize*. */
  readonly keptIn: KeptIn | undefined;
  readonly choices: CustomizeChoices;
}

/**
 * *Customize*: the window that sets the columns, the rows and the card limits, and *Customize › Card*,
 * the way to the layout *Card* for people who may change layouts. Both are quiet buttons, like *Refresh*.
 */
export function CustomizeButton({ keptIn, view, columns, choices }: CustomizeButtonProps) {
  const [open, setOpen] = useState(false);
  const change = useChangeSearch(keptIn?.id ?? "");
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
      <CustomizeCardButton />
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
