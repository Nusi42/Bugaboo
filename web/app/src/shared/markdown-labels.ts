import { t } from "@bugaboo/messages";
import type { MarkdownEditorLabels } from "@bugaboo/ui";

/** The words of the Markdown editor, in the language of the person. */
export function markdownLabels(): MarkdownEditorLabels {
  return {
    write: t("markdown.write"),
    preview: t("markdown.preview"),
    toolbar: t("markdown.toolbar"),
    bold: t("markdown.bold"),
    italic: t("markdown.italic"),
    list: t("markdown.list"),
    link: t("markdown.link"),
    code: t("markdown.code"),
    quote: t("markdown.quote"),
    linkText: t("markdown.link-text"),
    previewEmpty: t("markdown.preview-empty"),
    previewFailed: t("markdown.preview-failed"),
  };
}
