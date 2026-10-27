import { doNothing } from "@bugaboo/ui";

import { chooseColumns, moveColumn } from "../../shared/list-columns.ts";
import { ColumnRow } from "./ColumnRow.tsx";
import type { ColumnChoice } from "./columns-choice.ts";

export interface ColumnListProps {
  /** The columns now, in order. */
  readonly columns: readonly string[];
  /** The fields the person may read. */
  readonly readable: readonly ColumnChoice[];
  readonly onChange: (columns: readonly string[]) => void;
}

/** The ticked columns in their order, then the fields that are not columns. */
export function ColumnList({ columns, readable, onChange }: ColumnListProps) {
  const nameOf = (word: string): string =>
    readable.find((choice) => choice.word === word)?.name ?? word;
  const tick = (word: string, on: boolean): void => {
    const ticked = on ? [...columns, word] : columns.filter((each) => each !== word);
    onChange(
      chooseColumns(
        ticked,
        readable.map((choice) => choice.word),
      ),
    );
  };
  return (
    <>
      {columns.map((word, index) => (
        <ColumnRow
          key={word}
          word={word}
          name={nameOf(word)}
          place={{ index, last: columns.length - 1 }}
          onTick={(on) => {
            tick(word, on);
          }}
          onMove={(direction) => {
            onChange(moveColumn(columns, word, direction));
          }}
        />
      ))}
      {readable
        .filter((choice) => !columns.includes(choice.word))
        .map((choice) => (
          <ColumnRow
            key={choice.word}
            word={choice.word}
            name={choice.name}
            onTick={(on) => {
              tick(choice.word, on);
            }}
            onMove={doNothing}
          />
        ))}
    </>
  );
}
