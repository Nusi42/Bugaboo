import { t } from "@bugaboo/messages";
import { Alert, alertTone, LongPreview, partOf, part } from "@bugaboo/ui";

import { keyedLiftSentencesOfPreview, personSentencesOfPreview } from "./preview-sentences.ts";
import type { Preview } from "./use-projects.ts";

/**
 * What a move would do, in plain sentences (*Show, then do*): who gains or loses access or changes
 * role, what else changes, and, when the move is not allowed, why.
 */
export function MovePreviewView({ preview }: { readonly preview: Preview }) {
  const groups = personSentencesOfPreview(preview);
  const lifts = keyedLiftSentencesOfPreview(preview);
  return (
    <section {...partOf(part.projectsPreview)} aria-label={t("projects.preview-heading")}>
      <h3 {...partOf(part.definition)}>{t("projects.preview-heading")}</h3>
      {preview.blocked ? (
        <Alert tone={alertTone.danger}>
          <p {...partOf(part.projectsAlertLine)}>{t("projects.preview-blocked")}</p>
          {lifts.map((line) => (
            <p key={line.key} {...partOf(part.projectsAlertLine)}>
              {line.text}
            </p>
          ))}
          {preview.blocked_out_of_sight ? (
            <p {...partOf(part.projectsAlertLine)}>{t("projects.preview-blocked-out-of-sight")}</p>
          ) : null}
        </Alert>
      ) : null}
      <LongPreview
        summary={t("projects.preview-summary", { changed: preview.summary.will_change })}
        label={t("projects.preview-list")}
        nothing={t("projects.preview-nothing")}
        showAll={(count) => t("common.show-all", { count })}
        showFewer={t("common.show-fewer")}
        groups={groups.map((group) => ({
          key: group.key,
          heading: <strong>{group.person}</strong>,
          lines: group.lines.map((line) => ({ key: line.key, content: line.text })),
        }))}
      />
      {preview.truncated ? <p {...partOf(part.muted)}>{t("projects.preview-truncated")}</p> : null}
      <p {...partOf(part.muted)}>{t("projects.preview-hidden-note")}</p>
    </section>
  );
}
