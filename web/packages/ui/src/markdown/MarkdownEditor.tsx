import { useState, type ClipboardEvent } from "react";
import {
  Label,
  Tab,
  TabList,
  TabPanel,
  Tabs,
  TextArea,
  TextField as AriaTextField,
} from "react-aria-components";

import { partOf } from "../parts.ts";
import { passPastedFiles } from "../pasted-files.ts";
import { busyState, fieldShape } from "../props.ts";
import { part } from "../tokens.ts";
import { FormattingToolbar } from "./FormattingToolbar.tsx";
import type { MarkdownEditorLabels } from "./labels.ts";
import { MarkdownPreview } from "./MarkdownPreview.tsx";
import { useFormatting } from "./use-format.ts";

/** The names of the two tabs: words of the component, not text for people. */
const tabs = { write: "write", preview: "preview" } as const;

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
function WriteBox({
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

/**
 * A text box for Markdown with a small toolbar (bold, italic, list, link, code, quote) and a
 * *Preview* tab. The preview is made by `bb-markdown` in WebAssembly, the very code the server
 * runs, loaded when the tab is first opened (ADR-0011); there is no other renderer in the
 * browser. The text is Markdown, so a pasted `<script>` shows as text.
 */
export function MarkdownEditor({ imageHosts, ...box }: MarkdownEditorProps) {
  const [tab, setTab] = useState<string>(tabs.write);
  return (
    <Tabs
      selectedKey={tab}
      onSelectionChange={(key) => {
        setTab(String(key));
      }}
    >
      <div {...partOf(part.markdownEditor)}>
        <TabList aria-label={box.label} {...partOf(part.markdownTabs)}>
          <Tab id={tabs.write} {...partOf(part.markdownTab)}>
            {box.labels.write}
          </Tab>
          <Tab id={tabs.preview} {...partOf(part.markdownTab)}>
            {box.labels.preview}
          </Tab>
        </TabList>
        <TabPanel id={tabs.write} {...partOf(part.markdownPanel)}>
          <WriteBox {...box} />
        </TabPanel>
        <TabPanel id={tabs.preview} {...partOf(part.markdownPanel)}>
          <div {...partOf(part.markdownPreview)}>
            <MarkdownPreview text={box.value} imageHosts={imageHosts} labels={box.labels} />
          </div>
        </TabPanel>
      </div>
    </Tabs>
  );
}
