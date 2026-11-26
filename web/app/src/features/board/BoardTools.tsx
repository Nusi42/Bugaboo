import { t } from "@bugaboo/messages";
import { Button, buttonLook, part, partOf, selectLook } from "@bugaboo/ui";

import type { ListView } from "../../shared/list-view.ts";
import { ViewChoices, type SearchFieldsAnswer } from "../search/index.ts";
import { refreshBoards } from "./board-data.ts";
import { CustomizeButton, type CustomizeButtonProps } from "./CustomizeButton.tsx";

export interface BoardToolsProps extends CustomizeButtonProps {
  readonly onView: (view: ListView) => void;
  readonly fields: SearchFieldsAnswer;
  /** A phone has *Show as* in the sheet of the filters. */
  readonly phone: boolean;
}

/**
 * The tools over the board (UX §4.6): *Show as*, *Customize* for the maker of the saved search, and
 * *Refresh*, all of one look. A board reads again after a change of its own and when the window gets
 * the focus again; *Refresh* is for what others changed meanwhile.
 */
export function BoardTools({ view, onView, fields, phone, ...rest }: BoardToolsProps) {
  return (
    <div {...partOf(part.boardTools)}>
      {phone ? null : (
        <ViewChoices view={view} fields={fields} onView={onView} look={selectLook.inline} />
      )}
      <CustomizeButton view={view} {...rest} />
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
