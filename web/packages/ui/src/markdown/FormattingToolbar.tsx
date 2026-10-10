import { Toolbar } from "react-aria-components";

import { Button } from "../Button.tsx";
import { partOf } from "../parts.ts";
import { buttonLook } from "../props.ts";
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
        <Button
          key={action}
          look={buttonLook.quiet}
          isDisabled={isDisabled}
          onPress={() => {
            onFormat(action);
          }}
        >
          {names[action]}
        </Button>
      ))}
    </Toolbar>
  );
}
