import { t } from "@bugaboo/messages";
import { Alert, alertTone, Button, partOf, part } from "@bugaboo/ui";

import { messageOf } from "../../shared/errors.ts";
import { editedThing } from "../../shared/editing/edited-thing.ts";
import { LostEditNotice } from "../../shared/editing/LostEditNotice.tsx";
import { HeldEditNotice } from "../../shared/editing/HeldEditNotice.tsx";
import { personName } from "../../shared/person.ts";
import { Offered } from "./DescriptionDraft.tsx";
import type { Description } from "./use-description.ts";
import { useWhoChanged } from "./use-who-changed.ts";

export interface DescriptionEditNoticesProps {
  readonly description: Description;
  /** The ID of the issue and the part of it, to find out who changed the text. */
  readonly issue: string;
  readonly which: string;
  /** Whether *Compare and save* or *Show changes* is open. */
  readonly comparing: boolean;
  readonly onCompare: () => void;
}

/** The note that tells why the text is *Not saved yet*, with the way on. */
function Lost({ description, onCompare }: DescriptionEditNoticesProps) {
  const { phase } = description;
  if (phase.kind !== "lost") {
    return null;
  }
  return (
    <LostEditNotice reason={phase.reason} by={phase.by} thing={editedThing.description}>
      <Button onPress={onCompare}>{t("issues.compare-and-save")}</Button>
    </LostEditNotice>
  );
}

/** Somebody else is editing the text: who, since when, and the things the person can do. */
function Held({ description }: Pick<DescriptionEditNoticesProps, "description">) {
  const { phase } = description;
  if (phase.kind !== "held") {
    return null;
  }
  return (
    <HeldEditNotice
      held={phase.lock}
      thing={editedThing.description}
      onAskToFinish={description.askToFinish}
      onEditAnyway={description.editAnyway}
    />
  );
}

/** The text changed after the person started: who changed it, if the page can tell, and the way to see the changes. */
function ChangedAfter({
  issue,
  which,
  version,
  onCompare,
}: Pick<DescriptionEditNoticesProps, "issue" | "which" | "onCompare"> & {
  readonly version: number;
}) {
  const name = useWhoChanged(issue, which, version);
  return (
    <Alert tone={alertTone.info}>
      <div {...partOf(part.issueNote)}>
        <span>
          {name === undefined
            ? t("issues.changed-after-you")
            : t("issues.changed-after-you-by", { name })}
        </span>
        <Button onPress={onCompare}>{t("issues.show-changes")}</Button>
      </div>
    </Alert>
  );
}

/** What the person needs to know while they edit: a draft, a request, a change, a failure. */
export function DescriptionEditNotices(props: DescriptionEditNoticesProps) {
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
        <ChangedAfter
          issue={props.issue}
          which={props.which}
          version={description.version}
          onCompare={onCompare}
        />
      ) : null}
      <Lost {...props} />
      {problem === null || problem === undefined ? null : (
        <Alert tone={alertTone.danger}>{messageOf(problem)}</Alert>
      )}
    </>
  );
}
