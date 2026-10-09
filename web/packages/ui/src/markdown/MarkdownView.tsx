import type { SafeHtml } from "@bugaboo/api-client";

import { partOf } from "../parts.ts";
import { part } from "../tokens.ts";
import { useHtml } from "./policy.ts";

export interface MarkdownViewProps {
  /** Safe HTML from `bb-markdown`: from the server, or from the preview of the editor. */
  readonly html: SafeHtml;
}

/**
 * Shows the text of a person. This is the one place in the app that sets HTML from a string
 * (SEC-RICHTEXT), through the Trusted Types policy that the content security policy names. The
 * HTML is never made here: `bb-markdown` makes it safe, in Rust, on the server and in WebAssembly.
 */
export function MarkdownView({ html }: MarkdownViewProps) {
  return <div ref={useHtml(html)} {...partOf(part.markdown)} />;
}
