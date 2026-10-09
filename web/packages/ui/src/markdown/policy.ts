import { useLayoutEffect, useRef, type RefObject } from "react";

/**
 * The one Trusted Types policy of the app (SEC-13, ADR-0011). The content security policy says
 * `trusted-types bugaboo-markdown`, so a browser refuses to set HTML from a string by any other
 * way. SEC-RICHTEXT holds this name to the name in the header of the server.
 */
export const trustedTypesPolicyName = "bugaboo-markdown";

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
export function useHtml(html: string): RefObject<HTMLDivElement | null> {
  const box = useRef<HTMLDivElement>(null);
  useLayoutEffect(() => {
    if (box.current !== null) {
      box.current.innerHTML = trustedHtml(html);
    }
  }, [html]);
  return box;
}
