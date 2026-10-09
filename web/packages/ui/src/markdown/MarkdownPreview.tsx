import { Alert } from "../Alert.tsx";
import { partOf } from "../parts.ts";
import { alertTone } from "../props.ts";
import { part } from "../tokens.ts";
import type { MarkdownEditorLabels } from "./labels.ts";
import { MarkdownView } from "./MarkdownView.tsx";
import { useMarkdownPreview } from "./use-preview.ts";

export interface MarkdownPreviewProps {
  /** The Markdown to show as others will see it. */
  readonly text: string;
  readonly imageHosts: readonly string[];
  readonly labels: MarkdownEditorLabels;
}

/** What the preview shows: the text as others will see it, a hint for an empty text, or a problem. */
export function MarkdownPreview({ text, imageHosts, labels }: MarkdownPreviewProps) {
  const preview = useMarkdownPreview(text, imageHosts);
  if (preview.state === "failed") {
    return <Alert tone={alertTone.danger}>{labels.previewFailed}</Alert>;
  }
  if (preview.state === "loading") {
    return null;
  }
  return text.trim() === "" ? (
    <p {...partOf(part.muted)}>{labels.previewEmpty}</p>
  ) : (
    <MarkdownView html={preview.html} />
  );
}
