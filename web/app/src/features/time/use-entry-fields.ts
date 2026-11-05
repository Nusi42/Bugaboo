import { timeEntryFieldsFor, useFields, type OwnField } from "../fields/index.ts";
import { useTimer } from "./use-timer.ts";

/**
 * The own fields a form of a time entry shows for an issue of a project and a type: the starter
 * fields (*Activity* and *Cost center*, found by the mark the server gives them and never by their
 * name), then any other field of time entries that must be filled in. Fields the person may not see
 * are not in the list the server sends.
 */
export function useEntryFields(
  project: string | undefined,
  issueType: string | undefined,
): readonly OwnField[] {
  const fields = useFields({ project }).data?.fields ?? [];
  const starters = Object.values(useTimer().data?.timer.starters ?? {});
  const reaching = timeEntryFieldsFor(fields, issueType);
  const starter = starters.flatMap((id) => reaching.filter((field) => field.id === id));
  const required = reaching.filter(
    (field) => field.rules.required === true && !starters.includes(field.id),
  );
  return [...starter, ...required];
}
