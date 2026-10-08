import { t } from "@bugaboo/messages";
import { useState } from "react";

import { messageOf } from "../../shared/errors.ts";
import { rememberChoice } from "../../shared/recent-choices.ts";
import type { Choice, Subject } from "./assignments.ts";
import type { RowSpec } from "./use-row-changes.ts";
import { type SubjectKind } from "../../shared/person.ts";

/** The word that stands for *No access* among the roles. */
export const NO_ACCESS = "no-access";
/** The word that stands for nothing chosen yet. */
export const NOTHING = "";

/** What the form of *Give a role* keeps and does. */
export interface GiveForm {
  readonly kind: SubjectKind;
  readonly setKind: (kind: SubjectKind) => void;
  readonly who: Subject | null;
  readonly pick: (choice: Choice) => void;
  readonly role: string;
  readonly setRole: (role: string) => void;
  readonly reach: RowSpec["appliesTo"];
  readonly setReach: (reach: RowSpec["appliesTo"]) => void;
  readonly problem: string | null;
  /** Gives the row, when whom and which role are chosen; says what is missing when not. */
  readonly submit: () => void;
}

/**
 * The state of the form of *Give a role*: whom (a person or a group), the role (or *No access*) and
 * where it counts. *No access* always covers the project and its sub-projects, so the reach that
 * was chosen is left out for it.
 */
export function useGiveForm(
  give: (spec: RowSpec) => Promise<boolean>,
  preset: Subject | undefined,
): GiveForm {
  const [kind, setKind] = useState<SubjectKind>(preset?.kind ?? "person");
  const [who, setWho] = useState<Subject | null>(preset ?? null);
  const [role, setRole] = useState(NOTHING);
  const [reach, setReach] = useState<RowSpec["appliesTo"]>("project-and-sub-projects");
  const [problem, setProblem] = useState<string | null>(null);
  const submit = (): void => {
    if (who === null || role === NOTHING) {
      setProblem(t("assignments.incomplete"));
      return;
    }
    const noAccess = role === NO_ACCESS;
    give({
      who,
      role: noAccess ? null : role,
      appliesTo: noAccess ? "project-and-sub-projects" : reach,
    })
      .then((given) => {
        if (given) {
          rememberChoice(who.kind, who.id);
          setProblem(null);
          setWho(null);
          setRole(NOTHING);
        }
      })
      .catch((error: unknown) => {
        setProblem(messageOf(error));
      });
  };
  return {
    kind,
    setKind: (next) => {
      setKind(next);
      setWho(null);
    },
    who,
    pick: (choice) => {
      setWho({ kind, id: choice.id, name: choice.name });
    },
    role,
    setRole,
    reach,
    setReach,
    problem,
    submit,
  };
}
