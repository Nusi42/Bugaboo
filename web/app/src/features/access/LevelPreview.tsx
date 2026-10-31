import { t } from "@bugaboo/messages";
import { Card, cardLook, partOf, part } from "@bugaboo/ui";

import { previewLines, type LevelPreviewProps } from "./level-preview.ts";

/**
 * What a person with the chosen role sees, under a table of levels (SEC-04, PRIV-12): level by level from the highest,
 * every field or kind the role has at that level. *Field permissions* and *Who sees people* show it the same way, so an
 * administrator reads the effect of a column before anybody else does (M5-F135).
 */
export function LevelPreview<Subject extends string, Level extends string>(
  props: LevelPreviewProps<Subject, Level>,
) {
  const name = props.table.roles.find((known) => known.id === props.role)?.name ?? "";
  return (
    // On a phone it is no box inside the box of the page: one padding, the full width for its words.
    <Card heading={t("access.preview-heading", { role: name })} look={cardLook.openOnPhone}>
      <ul {...partOf(part.list, { variants: ["tight"] })}>
        {previewLines(props).map((line) => (
          <li key={line.level} {...partOf(part.accessPreviewLine)}>
            {line.text}
          </li>
        ))}
      </ul>
    </Card>
  );
}
