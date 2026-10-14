// What a test that cuts the network of one browser on purpose expects that browser to write to its
// console. Kept apart from the fixtures so that the rules are tested on their own
// (`web/tooling/tolerance.test.ts`).

/** The way a test cuts a network and brings it back; the lines are expected only in between. */
export interface Cut {
  readonly cut: () => void;
  readonly restored: () => void;
}

/** A network that is cut on purpose, and the console lines expected from it. */
interface Tolerance {
  readonly id: symbol;
  readonly context: unknown;
  readonly lines: Readonly<RegExp>;
}

/**
 * The expected console lines of a test: a line is expected when it is written by the context a
 * test named, while that context is cut off, and it matches the lines the test named. Nothing
 * else is: not the lines of another context, not a line while the context is online, and not an
 * uncaught error (the fixtures never ask about those).
 */
export class Tolerances {
  private readonly all: Tolerance[] = [];
  /** Which of them are cut off now. */
  private readonly offline = new Set<symbol>();

  /** Expects the lines of the context, from `cut` until `restored`. */
  add(context: unknown, lines: Readonly<RegExp>): Cut {
    const id = Symbol("cut network");
    this.all.push({ id, context, lines });
    return {
      cut: () => {
        this.offline.add(id);
      },
      restored: () => {
        this.offline.delete(id);
      },
    };
  }

  /** Whether a test expects this line of this context now. */
  expects(context: unknown, line: string): boolean {
    return this.all.some(
      (each) => each.context === context && this.offline.has(each.id) && each.lines.test(line),
    );
  }

  /** Forgets everything, so one test never lets another off. */
  clear(): void {
    this.all.length = 0;
    this.offline.clear();
  }
}
