const safeHtmlBrand: unique symbol = Symbol("safe-html");

/**
 * HTML that `bb-markdown` wrote and made safe (SEC-13). `MarkdownView` takes this type and no
 * plain string, so the one place that sets HTML from a string cannot be given any other text.
 */
export interface SafeHtml {
  readonly html: string;
  readonly [safeHtmlBrand]: true;
}

/**
 * What `markSafeHtml` makes. It is a class so that it is not a plain object: the query cache
 * shares structure only between plain objects and would copy a plain one without its mark.
 */
class MadeSafeHtml implements SafeHtml {
  readonly html: string;
  readonly [safeHtmlBrand] = true as const;
  constructor(html: string) {
    this.html = html;
  }
}

/** Every value `markSafeHtml` made. A look-alike object, or a cast, is not in it. */
const made = new WeakSet<object>();

/**
 * Says that `html` is the result of `bb-markdown`. Only the code that gets such HTML may call this:
 * the WebAssembly renderer, and the client when the server sends a rendered field (SEC-RICHTEXT
 * holds the list of callers).
 */
export function markSafeHtml(html: string): SafeHtml {
  const safe = Object.freeze(new MadeSafeHtml(html));
  made.add(safe);
  return safe;
}

/** Whether `value` was made by `markSafeHtml`, which a type alone cannot say. */
export function isSafeHtml(value: unknown): value is SafeHtml {
  return typeof value === "object" && value !== null && made.has(value);
}
