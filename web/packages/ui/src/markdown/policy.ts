import { isSafeHtml, trustedTypesPolicyName, type SafeHtml } from "@bugaboo/api-client";
import { useLayoutEffect, useRef, type RefObject } from "react";

/*
 * The one Trusted Types policy of the app (SEC-13, ADR-0011). Its name is written once, in
 * `bb-api-words`, where the content security policy of the server takes it from too; a browser
 * refuses to set HTML from a string by any other way.
 */

/**
 * The part of the Trusted Types interface that is used here (the compiler's own list of types does
 * not have it yet). A value the policy makes is `TrustedHTML`; the browser takes it where it takes
 * a string, so it is typed as the string it stands for.
 */
interface TrustedTypes {
  createPolicy(
    name: string,
    rules: { readonly createHTML: (written: string) => string },
  ): { readonly createHTML: (written: string) => string };
}

function isTrustedTypes(value: unknown): value is TrustedTypes {
  return (
    typeof value === "object" &&
    value !== null &&
    "createPolicy" in value &&
    typeof value.createPolicy === "function"
  );
}

let policy: ReturnType<TrustedTypes["createPolicy"]> | undefined;

/**
 * The HTML as the browser wants it. The text that comes here is only ever what `bb-markdown`
 * wrote, on the server or in the browser's WebAssembly: it is made safe there, so this policy
 * hands the string on as it is. A browser without Trusted Types takes the plain string.
 */
function trustedHtml(html: string): string {
  const types: unknown = Reflect.get(globalThis, "trustedTypes");
  if (!isTrustedTypes(types)) {
    return html;
  }
  policy ??= types.createPolicy(trustedTypesPolicyName, { createHTML: (written) => written });
  return policy.createHTML(html);
}

/**
 * The element to put HTML in: its content is the HTML, set through the policy. This is the one
 * place in the app that sets HTML from a string (SEC-RICHTEXT); only `MarkdownView` uses it.
 */
export function useHtml(safe: SafeHtml): RefObject<HTMLDivElement | null> {
  const box = useRef<HTMLDivElement>(null);
  const html = isSafeHtml(safe) ? safe.html : "";
  useLayoutEffect(() => {
    if (box.current !== null) {
      box.current.innerHTML = trustedHtml(html);
    }
  }, [html]);
  return box;
}
