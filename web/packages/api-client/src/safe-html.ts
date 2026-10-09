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
 * Says that `html` is the result of `bb-markdown`. Only the code that gets such HTML may call this:
 * the WebAssembly renderer, and the client when the server sends a rendered field (SEC-RICHTEXT
 * holds the list of callers).
 */
export function markSafeHtml(html: string): SafeHtml {
  return { html, [safeHtmlBrand]: true };
}
