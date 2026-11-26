import { t } from "@bugaboo/messages";
import { buttonType, Icon, iconName, iconSize, part, partOf } from "@bugaboo/ui";
import { useState, type ReactNode } from "react";

export interface BoardRowProps {
  /** What the row is called, and how many issues it holds. */
  readonly name: string;
  readonly count: number;
  /** The cells of the row: all the columns on a computer, the one open column on a phone. */
  readonly children: ReactNode;
}

/** One row of the board (UX §4.6): a header that folds it, with its name and its count, and its cells. */
export function BoardRow({ name, count, children }: BoardRowProps) {
  const [folded, setFolded] = useState(false);
  return (
    <section {...partOf(part.boardRow)} aria-label={name}>
      <button
        type={buttonType.button}
        aria-expanded={!folded}
        {...partOf(part.boardRowToggle)}
        onClick={() => {
          setFolded((now) => !now);
        }}
      >
        <Icon name={folded ? iconName.next : iconName.down} size={iconSize.small} />
        {name}
        <span {...partOf(part.boardRowCount)}>{t("board.count", { count })}</span>
      </button>
      {folded ? null : children}
    </section>
  );
}
