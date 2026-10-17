import { t } from "@bugaboo/messages";
import { Alert, alertTone, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { CheckAnswer } from "./CheckAnswer.tsx";
import { IssueTypeQuestion } from "./IssueTypeQuestion.tsx";
import { PersonPicker, PlacePicker } from "./Pickers.tsx";
import { useCheckAccess } from "./use-check.ts";

export interface CheckAccessViewProps {
  /** The project to look in (an ID), when the page belongs to one; else the person picks the place. */
  readonly project?: string;
}

/**
 * *Check access* (SEC-03): choose where to look (on the organization's page) and a person, and read
 * what they can do there, every field and how they see people, each with the reason in a sentence.
 */
export function CheckAccessView({ project }: CheckAccessViewProps) {
  const [person, setPerson] = useState<string>();
  const [place, setPlace] = useState<string | null>(null);
  const [issueType, setIssueType] = useState<string>();
  const where = project ?? place ?? undefined;
  const answer = useCheckAccess(person, where, issueType);
  return (
    <div {...partOf(part.stack)}>
      {project === undefined ? <PlacePicker value={place} onChange={setPlace} /> : null}
      <PersonPicker
        project={where}
        value={person}
        onChange={(next) => {
          setPerson(next);
          setIssueType(undefined);
        }}
      />
      {person === undefined ? (
        <p {...partOf(part.muted)}>{t("check-access.pick-a-person")}</p>
      ) : null}
      {answer.isError ? <Alert tone={alertTone.danger}>{messageOf(answer.error)}</Alert> : null}
      {answer.data?.differs_by_type === true ? (
        <IssueTypeQuestion value={issueType} onChange={setIssueType} />
      ) : null}
      {answer.data === undefined ? null : <CheckAnswer answer={answer.data} />}
    </div>
  );
}
