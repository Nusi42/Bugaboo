import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";

import type { IssueLine } from "./issues.ts";

export interface SlimIssueListProps {
  readonly lines: readonly IssueLine[];
  /** The issue that is open, by its issue ID. */
  readonly open: string;
  readonly onOpen: (key: string) => void;
}

/** The slim list on the left of the focus view (UX §3.4): the issue ID and title of each issue of the project. */
export function SlimIssueList({ lines, open, onOpen }: SlimIssueListProps) {
  return (
    <ul {...partOf(part.list, { variants: ["tight"] })} aria-label={t("issues.list")}>
      {lines.map((line) => (
        <li key={line.id}>
          <Button
            look={line.key === open ? buttonLook.plain : buttonLook.quiet}
            current={line.key === open}
            onPress={() => {
              onOpen(line.key);
            }}
          >
            {t("issues.title-of", { key: line.key, title: line.title })}
          </Button>
        </li>
      ))}
    </ul>
  );
}
