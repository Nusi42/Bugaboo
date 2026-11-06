import { useState } from "react";

import { readBreakLines } from "./sheet-model.ts";
import type { DaysProps } from "./sheet-props.ts";

/**
 * Saves the lines of the window of the breaks the way the cell *Breaks* saves what is typed: it
 * reads them (`readBreakLines`) and the cells of the day save them (with the reason a past day asks
 * for). The window closes when it is saved; the sentence of what is wrong stays in it.
 */
export function useSaveBreaks(date: string, editing: DaysProps["editing"], onSaved: () => void) {
  const [problem, setProblem] = useState<string | undefined>();
  const [saving, setSaving] = useState(false);
  const save = async (lines: string): Promise<void> => {
    const read = readBreakLines(lines.split(/\r?\n/u));
    if ("mistake" in read) {
      setProblem(read.mistake);
      return;
    }
    setSaving(true);
    const done = await editing.editDay(date, "breaks", "value" in read ? read.value : []);
    setSaving(false);
    if (done.problem === undefined) {
      onSaved();
    } else {
      setProblem(done.problem);
    }
  };
  return { save, problem, saving };
}
