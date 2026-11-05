import { IssueId, Length, SentenceWithValues } from "@bugaboo/ui";

export interface IdsInProps {
  /** The sentence as the catalog writes it, with the IDs and the lengths in it. */
  readonly sentence: string;
  /** The Issue IDs in the sentence: drawn as IDs, in the code font. */
  readonly ids?: readonly string[];
  /** The lengths and times of day in the sentence: drawn in the code font too. */
  readonly lengths?: readonly string[];
}

/** No values in a sentence. */
const none: readonly string[] = [];

/**
 * A sentence of the timer or the work day with its Issue IDs, lengths and times of day in the code
 * face (UX §7.3: IDs and times use the mono face); the words around them are as they are.
 */
export function IdsIn({ sentence, ids = none, lengths = none }: IdsInProps) {
  return (
    <SentenceWithValues sentence={sentence} values={[...ids, ...lengths]}>
      {(value) => (ids.includes(value) ? <IssueId id={value} /> : <Length text={value} />)}
    </SentenceWithValues>
  );
}
