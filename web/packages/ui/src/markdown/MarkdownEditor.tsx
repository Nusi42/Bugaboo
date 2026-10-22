import { useState } from "react";

import { partOf } from "../parts.ts";
import { Tabs } from "../Tabs.tsx";
import { part } from "../tokens.ts";
import { MarkdownPreview } from "./MarkdownPreview.tsx";
import { WriteBox, type MarkdownEditorProps } from "./MarkdownWriteBox.tsx";

/** The names of the two tabs: words of the component, not text for people. */
const tabs = { write: "write", preview: "preview" } as const;

export type { MarkdownEditorLabels } from "./labels.ts";
export type { MarkdownEditorProps } from "./MarkdownWriteBox.tsx";

/**
 * A text box for Markdown with a small toolbar (bold, italic, list, link, code, quote) and a
 * *Preview* tab. The preview is made by `bb-markdown` in WebAssembly, the very code the server
 * runs, loaded when the tab is first opened (ADR-0011); there is no other renderer in the
 * browser. The text is Markdown, so a pasted `<script>` shows as text.
 */
export function MarkdownEditor({ imageHosts, ...box }: MarkdownEditorProps) {
  const [tab, setTab] = useState<(typeof tabs)[keyof typeof tabs]>(tabs.write);
  return (
    <div {...partOf(part.markdownEditor)}>
      <Tabs
        label={box.label}
        tabs={[
          { id: tabs.write, label: box.labels.write },
          { id: tabs.preview, label: box.labels.preview },
        ]}
        selected={tab}
        onSelect={setTab}
      >
        {(id) =>
          id === tabs.write ? (
            <WriteBox {...box} />
          ) : (
            <div {...partOf(part.markdownPreview)}>
              <MarkdownPreview text={box.value} imageHosts={imageHosts} labels={box.labels} />
            </div>
          )
        }
      </Tabs>
    </div>
  );
}
