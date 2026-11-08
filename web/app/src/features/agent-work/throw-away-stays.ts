/** What of a skipped field the preview of a throw away reads: the issue it is on and the field. */
export interface SkippedField {
  readonly issue?: { readonly id: string } | null;
  readonly field: string;
}

/** Whether the preview names this field of this issue among what stays when the run is undone. */
export function staysField(
  skipped: readonly SkippedField[],
  issue: string,
  field: string,
): boolean {
  return skipped.some((skip) => skip.issue?.id === issue && skip.field === field);
}
