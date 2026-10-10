import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, buttonLook, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { formatAgo, formatSince } from "../../shared/format.ts";
import { personName } from "../../shared/person.ts";
import { useNow } from "../../shared/use-now.ts";
import type { Description } from "./use-description.ts";

export interface DescriptionNotesProps {
  readonly description: Description;
  /** Whether *Compare and save* or *Show changes* is open. */
  readonly comparing: boolean;
  readonly onCompare: () => void;
}

/** The note that tells why the text is *Not saved yet*, with the way on. */
function Lost({ description, onCompare }: DescriptionNotesProps) {
  const { phase } = description;
  if (phase.kind !== "lost") {
    return null;
  }
  const name = phase.by === undefined ? t("issues.someone") : personName(phase.by);
  const why = {
    "ran-out": t("issues.lost-ran-out"),
    "taken-over": t("issues.lost-taken-over", { name }),
    "version-changed": t("issues.lost-changed", { name }),
  } as const;
  return (
    <Alert tone={alertTone.warning}>
      <div {...partOf(part.issueNote)}>
        <span>{why[phase.reason]}</span>
        <Button onPress={onCompare}>{t("issues.compare-and-save")}</Button>
      </div>
    </Alert>
  );
}

/** Somebody else is editing the text: who, since when, and the two things the person can do. */
function Held({ description }: Pick<DescriptionNotesProps, "description">) {
  const { phase } = description;
  const now = useNow();
  if (phase.kind !== "held") {
    return null;
  }
  const { lock } = phase;
  const name = personName(lock.holder);
  return (
    <Alert tone={alertTone.info}>
      <div {...partOf(part.issueNote)}>
        <span>
          {lock.yours_elsewhere
            ? t("issues.editing-elsewhere")
            : t("issues.is-editing", { name, since: formatSince(lock.since, now) })}
        </span>
        {lock.yours_elsewhere ? null : (
          <Button onPress={description.askToFinish}>{t("issues.ask-to-finish", { name })}</Button>
        )}
        {lock.can_edit_anyway ? (
          <Button onPress={description.editAnyway}>{t("issues.edit-anyway")}</Button>
        ) : null}
      </div>
    </Alert>
  );
}

/** A draft that was found when the box opened: continue it or start again. */
function Offered({ description }: Pick<DescriptionNotesProps, "description">) {
  const { offered } = description;
  const now = useNow();
  if (offered === null) {
    return null;
  }
  const when = formatAgo(offered.savedAt, now);
  return (
    <Alert>
      <div {...partOf(part.issueNote)}>
        <span>{t("issues.draft-found", { when })}</span>
        <Button look={buttonLook.primary} onPress={description.continueDraft}>
          {t("issues.draft-continue", { when })}
        </Button>
        <Button onPress={description.dropDraft}>{t("issues.draft-drop")}</Button>
      </div>
    </Alert>
  );
}

/** What the person needs to know while they edit: a draft, a request, a change, a failure. */
export function DescriptionNotes(props: DescriptionNotesProps) {
  const { description, comparing, onCompare } = props;
  const { asked, phase, saved, startedFrom, problem } = description;
  const changedAfter = saved !== startedFrom && phase.kind !== "lost" && phase.kind !== "reading";
  return (
    <>
      <Offered description={description} />
      <Held description={description} />
      {asked === null ? null : (
        <Alert tone={alertTone.warning}>
          <div {...partOf(part.issueNote)}>
            <span>{t("issues.asked-to-finish", { name: personName(asked) })}</span>
            <Button onPress={description.save}>{t("common.save")}</Button>
          </div>
        </Alert>
      )}
      {changedAfter && !comparing ? (
        <Alert tone={alertTone.info}>
          <div {...partOf(part.issueNote)}>
            <span>{t("issues.changed-after-you")}</span>
            <Button onPress={onCompare}>{t("issues.show-changes")}</Button>
          </div>
        </Alert>
      ) : null}
      <Lost {...props} />
      {problem === null || problem === undefined ? null : (
        <Alert tone={alertTone.danger}>{messageOf(problem)}</Alert>
      )}
    </>
  );
}
