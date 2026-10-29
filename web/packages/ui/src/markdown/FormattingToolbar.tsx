import { Toolbar } from "react-aria-components";

import { IconButton } from "../IconButton.tsx";
import { iconName, type IconName } from "../icons.ts";
import { partOf } from "../parts.ts";
import { part } from "../tokens.ts";
import { formatAction, type FormatAction } from "./format.ts";
import type { MarkdownEditorLabels } from "./labels.ts";

/** The buttons of the toolbar, in the order they are shown. */
const toolbarActions: readonly FormatAction[] = [
  formatAction.bold,
  formatAction.italic,
  formatAction.list,
  formatAction.link,
  formatAction.code,
  formatAction.quote,
];

/** The drawing of each formatting button: quiet icons, named in words for screen readers and as a tooltip. */
const icons: Record<FormatAction, IconName> = {
  bold: iconName.bold,
  italic: iconName.italic,
  list: iconName.list,
  link: iconName.link,
  code: iconName.code,
  quote: iconName.quote,
};

/** The formatting buttons above the text box. */
export function FormattingToolbar({
  labels,
  onFormat,
  isDisabled = false,
}: {
  readonly labels: MarkdownEditorLabels;
  readonly onFormat: (action: FormatAction) => void;
  /** The text is being saved: no formatting until the answer is there. */
  readonly isDisabled?: boolean;
}) {
  const names: Record<FormatAction, string> = {
    bold: labels.bold,
    italic: labels.italic,
    list: labels.list,
    link: labels.link,
    code: labels.code,
    quote: labels.quote,
  };
  return (
    <Toolbar aria-label={labels.toolbar} {...partOf(part.markdownToolbar)}>
      {toolbarActions.map((action) => (
        <IconButton
          key={action}
          icon={icons[action]}
          label={names[action]}
          isDisabled={isDisabled}
          onPress={() => {
            onFormat(action);
          }}
        />
      ))}
    </Toolbar>
  );
}
