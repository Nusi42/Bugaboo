/** The texts of the parts that are loaded, by key. A part adds its texts when its file is loaded. */
const loaded = new Map<string, string>();

/** Called by a generated part (`cargo xtask gen messages`) when it is loaded. */
export function register(part: Readonly<Record<string, string>>): void {
  for (const [key, text] of Object.entries(part)) {
    loaded.set(key, text);
  }
}

/** The text of `key`, or `undefined` when its part is not loaded. */
export function textOf(key: string): string | undefined {
  return loaded.get(key);
}
