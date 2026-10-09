import { t } from "@bugaboo/messages";
import { Card, MarkdownEditor, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { markdownLabels } from "../../shared/markdown-labels.ts";

/** No image from another host: the hosts of the organization come with the screens that need them. */
const noImageHosts: readonly string[] = [];

/**
 * A text box for Markdown with its preview, on the page of the frames: the place to try how
 * descriptions and comments are written until the issue page uses the editor (UX 4.4).
 */
export function MarkdownSample() {
  const [text, setText] = useState("");
  return (
    <Card heading={t("frames.text-title")}>
      <p {...partOf(part.text)}>{t("frames.text-intro")}</p>
      <MarkdownEditor
        label={t("frames.text-label")}
        labels={markdownLabels()}
        value={text}
        onChange={setText}
        imageHosts={noImageHosts}
      />
    </Card>
  );
}
