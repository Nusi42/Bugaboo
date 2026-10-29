import { t } from "@bugaboo/messages";
import { Alert, alertTone, type LongPreviewLine, partOf, part } from "@bugaboo/ui";

import { AppLongPreview } from "../../shared/app-long-preview.tsx";
import {
  keyedLiftSentencesOfPreview,
  type PersonSentences,
  personSentencesOfPreview,
} from "./preview-sentences.ts";
import type { Preview } from "./use-projects.ts";

/** The lines under a person: the changes, or, where the person is changed in several projects, each project with its changes. */
function linesOf(group: PersonSentences): readonly LongPreviewLine[] {
  const [only, ...others] = group.projects;
  if (only !== undefined && others.length === 0) {
    return only.lines.map((line) => ({ key: line.key, content: line.text }));
  }
  return group.projects.map((project) => ({
    key: project.key,
    content: (
      <>
        <span {...partOf(part.muted)}>{project.project}</span>
        <ul {...partOf(part.longPreviewLines)}>
          {project.lines.map((line) => (
            <li key={line.key}>{line.text}</li>
          ))}
        </ul>
      </>
    ),
  }));
}

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
      <AppLongPreview
        summary={t("projects.preview-summary", { changed: preview.summary.will_change })}
        label={t("projects.preview-list")}
        groups={groups.map((group) => ({
          key: group.key,
          heading: <strong>{group.person}</strong>,
          lines: linesOf(group),
        }))}
      />
      {preview.truncated ? <p {...partOf(part.muted)}>{t("projects.preview-truncated")}</p> : null}
      <p {...partOf(part.muted)}>{t("projects.preview-hidden-note")}</p>
    </section>
  );
}
