import { t } from "@bugaboo/messages";
import { Button, buttonLook, partOf, part } from "@bugaboo/ui";
import { useState } from "react";

import { combine, compareRows, type CompareRow as CompareRowData, type Side } from "./compare.ts";
import { CompareRow } from "./CompareRow.tsx";

export interface CompareAndSaveProps {
  /** The text the person wrote. */
  readonly mine: string;
  /** The newer text that is saved now. */
  readonly theirs: string;
  /** Saves the chosen text. */
  readonly onSave: (text: string) => void;
  readonly onClose: () => void;
}

/** A key for a row: its place and its words, so a row that changed is drawn again. */
function keyOf(row: CompareRowData, at: number): string {
  return `${at.toString()}:${row.mine}:${row.theirs}`;
}

/**
 * *Compare and save* (ISS-17, UX §4.4): the person's text next to the newer version, paragraph by
 * paragraph. For each paragraph that differs the person picks one; the chosen text is saved. Nothing
 * is chosen for them: the default keeps their own text.
 */
export function CompareAndSave({ mine, theirs, onSave, onClose }: CompareAndSaveProps) {
  const rows = compareRows(mine, theirs);
  const [picks, setPicks] = useState<readonly Side[]>([]);
  const sideAt = (at: number): Side => picks[at] ?? "mine";
  const chosen = combine(
    rows,
    rows.map((_, at) => sideAt(at)),
  );
  return (
    <section {...partOf(part.issueCompare)} aria-label={t("issues.compare-title")}>
      <p {...partOf(part.text)}>{t("issues.compare-intro")}</p>
      {rows.map((row, at) => (
        <CompareRow
          key={keyOf(row, at)}
          row={row}
          number={at + 1}
          side={sideAt(at)}
          onPick={(side) => {
            setPicks(rows.map((_, index) => (index === at ? side : sideAt(index))));
          }}
        />
      ))}
      <div {...partOf(part.issueSectionActions)}>
        <Button
          look={buttonLook.primary}
          onPress={() => {
            onSave(chosen);
          }}
        >
          {t("issues.compare-save")}
        </Button>
        <Button
          onPress={() => {
            onSave(mine);
          }}
        >
          {t("issues.compare-save-mine")}
        </Button>
        <Button look={buttonLook.quiet} onPress={onClose}>
          {t("common.cancel")}
        </Button>
      </div>
    </section>
  );
}
