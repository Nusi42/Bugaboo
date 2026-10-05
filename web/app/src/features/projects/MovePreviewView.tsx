import { t } from "@bugaboo/messages";
import { Alert } from "@bugaboo/ui";

import { liftSentencesOfPreview, sentencesOfPreview } from "./preview-sentences.ts";
import styles from "./Projects.module.css";
import type { Preview } from "./use-projects.ts";

/**
 * What a move would do, in plain sentences (*Show, then do*): who gains or loses access or changes
 * role, what else changes, and, when the move is not allowed, why.
 */
export function MovePreviewView({ preview }: { readonly preview: Preview }) {
  const sentences = sentencesOfPreview(preview);
  const lifts = liftSentencesOfPreview(preview);
  return (
    <section className={styles.preview} aria-label={t("projects.preview-heading")}>
      <h3 className={styles.previewHeading}>{t("projects.preview-heading")}</h3>
      {preview.blocked ? (
        <Alert danger>
          <p className={styles.alertLine}>{t("projects.preview-blocked")}</p>
          {lifts.map((line) => (
            <p key={line} className={styles.alertLine}>
              {line}
            </p>
          ))}
        </Alert>
      ) : null}
      {sentences.length === 0 && !preview.blocked ? (
        <p className={styles.muted}>{t("projects.preview-nothing")}</p>
      ) : (
        <ul className={styles.sentences} aria-label={t("projects.preview-list")}>
          {sentences.map((line) => (
            <li key={line}>{line}</li>
          ))}
        </ul>
      )}
      {preview.truncated ? <p className={styles.muted}>{t("projects.preview-truncated")}</p> : null}
      <p className={styles.muted}>{t("projects.preview-hidden-note")}</p>
    </section>
  );
}
