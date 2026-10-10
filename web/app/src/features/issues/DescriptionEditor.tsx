import type { SafeHtml } from "@bugaboo/api-client";
import { t } from "@bugaboo/messages";
import { Button, buttonLook, MarkdownEditor, MarkdownView, partOf, part } from "@bugaboo/ui";

import { useImageHosts } from "../../shared/image-hosts.ts";
import { markdownLabels } from "../../shared/markdown-labels.ts";
import type { Description } from "./use-description.ts";

/** The box with the buttons under it. */
export function DescriptionEditor({ description }: { readonly description: Description }) {
  const hosts = useImageHosts();
  const { phase } = description;
  const saving = phase.kind === "saving";
  const held = phase.kind === "held";
  const cannotSave = held || phase.kind === "lost";
  return (
    <>
      {held ? null : (
        <MarkdownEditor
          label={t("issue-field.description")}
          labels={markdownLabels()}
          value={description.text}
          // Every key counts while the lock is being taken; only a save in flight takes none.
          onChange={description.type}
          busy={saving}
          imageHosts={hosts}
        />
      )}
      <div {...partOf(part.issueSectionActions)}>
        <Button
          look={buttonLook.primary}
          isPending={saving}
          isDisabled={cannotSave}
          onPress={description.save}
        >
          {t("common.save")}
        </Button>
        <Button onPress={description.cancel}>{t("common.cancel")}</Button>
      </div>
    </>
  );
}

export interface DescriptionReadingProps {
  /** The description as the server rendered it; nothing when the person may not see it. */
  readonly html: SafeHtml | undefined;
  /** Whether there is a description at all. */
  readonly filled: boolean;
}

/** The description as the server rendered it, or the word that there is none yet. */
export function DescriptionReading({ html, filled }: DescriptionReadingProps) {
  return (
    <div {...partOf(part.issueSectionReading)}>
      {html === undefined || !filled ? (
        <p {...partOf(part.muted)}>{t("issues.description.empty")}</p>
      ) : (
        <MarkdownView html={html} />
      )}
    </div>
  );
}
