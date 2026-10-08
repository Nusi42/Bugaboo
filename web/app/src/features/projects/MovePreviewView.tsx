import { t } from "@bugaboo/messages";
import { Alert, alertTone, partOf, part } from "@bugaboo/ui";

import { keyedLiftSentencesOfPreview, keyedSentencesOfPreview } from "./preview-sentences.ts";
import type { Preview } from "./use-projects.ts";

/**
 * What a move would do, in plain sentences (*Show, then do*): who gains or loses access or changes
 * role, what else changes, and, when the move is not allowed, why.
 */
export function MovePreviewView({ preview }: { readonly preview: Preview }) {
  const sentences = keyedSentencesOfPreview(preview);
  const lifts = keyedLiftSentencesOfPreview(preview);
  return (
    <section {...partOf(part.projectsPreview)} aria-label={t("projects.preview-heading")}>
      <h3 {...partOf(part.projectsPreviewHeading)}>{t("projects.preview-heading")}</h3>
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
      {sentences.length === 0 && !preview.blocked ? (
        <p {...partOf(part.muted)}>{t("projects.preview-nothing")}</p>
      ) : (
        <ul {...partOf(part.projectsSentences)} aria-label={t("projects.preview-list")}>
          {sentences.map((line) => (
            <li key={line.key}>{line.text}</li>
          ))}
        </ul>
      )}
      {preview.truncated ? <p {...partOf(part.muted)}>{t("projects.preview-truncated")}</p> : null}
      <p {...partOf(part.muted)}>{t("projects.preview-hidden-note")}</p>
    </section>
  );
}
