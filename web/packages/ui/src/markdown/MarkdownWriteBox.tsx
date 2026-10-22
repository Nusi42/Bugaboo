import { type ClipboardEvent } from "react";
import { Label, TextArea, TextField as AriaTextField } from "react-aria-components";

import { partOf } from "../parts.ts";
import { passPastedFiles } from "../pasted-files.ts";
import { busyState, fieldShape } from "../props.ts";
import { part } from "../tokens.ts";
import { FormattingToolbar } from "./FormattingToolbar.tsx";
import type { MarkdownEditorLabels } from "./labels.ts";
import { useFormatting } from "./use-format.ts";

export type { MarkdownEditorLabels } from "./labels.ts";

export interface MarkdownEditorProps {
  /** The visible name of the field, already in the person's language. */
  readonly label: string;
  readonly labels: MarkdownEditorLabels;
  readonly name?: string;
  /** The Markdown the person is writing. */
  readonly value: string;
  readonly onChange: (value: string) => void;
  /** The hosts from which images may be shown (the setting `content.image-hosts`). */
  readonly imageHosts: readonly string[];
  /** The text is being saved: the box shows it and takes no key until the answer is there. */
  readonly busy?: boolean;
  /**
   * Gets the files that are pasted into the box (a screenshot): they are taken out of the paste,
   * so no text is written for them. Without it a paste is left alone.
   */
  readonly onFiles?: (files: readonly File[]) => void;
}

/** The text box with its label and toolbar. */
export function WriteBox({
  label,
  labels,
  name = "",
  value,
  onChange,
  busy = false,
  onFiles,
}: Omit<MarkdownEditorProps, "imageHosts">) {
  const { area, format } = useFormatting(value, onChange, { linkText: labels.linkText });
  return (
    <AriaTextField
      {...partOf(part.field)}
      name={name}
      value={value}
      onChange={onChange}
      isReadOnly={busy}
    >
      <Label {...partOf(part.fieldLabel)}>{label}</Label>
      <FormattingToolbar labels={labels} onFormat={format} isDisabled={busy} />
      <TextArea
        ref={area}
        {...partOf(part.fieldInput, {
          variants: [fieldShape.multiline],
          states: [busy && busyState.busy],
        })}
        rows={8}
        aria-busy={busy}
        onPaste={(event: ClipboardEvent<HTMLTextAreaElement>) => {
          passPastedFiles(event, onFiles);
        }}
      />
    </AriaTextField>
  );
}
