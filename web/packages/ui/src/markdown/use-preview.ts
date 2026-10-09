import { useEffect, useMemo, useState } from "react";

/** What the preview has: the module is loading, it is there, or it could not be loaded. */
export type Preview =
  | { readonly state: "loading" }
  | { readonly state: "failed" }
  | { readonly state: "ready"; readonly html: string };

type Renderer = (text: string, imageHosts: readonly string[]) => string;

/** The module of `bb-wasm`, fetched the first time it is asked for. */
async function loadRenderer(): Promise<Renderer> {
  const { loadMarkdownRenderer } = await import("@bugaboo/wasm");
  return loadMarkdownRenderer();
}

/** What has been loaded so far. */
type Loaded =
  | { readonly state: "loading" }
  | { readonly state: "failed" }
  | { readonly state: "ready"; readonly render: Renderer };

/**
 * The HTML of a Markdown text, made by `bb-markdown` in WebAssembly. The module is loaded with
 * `import()` the first time a preview is shown, so a page without a preview never fetches it.
 */
export function useMarkdownPreview(text: string, imageHosts: readonly string[]): Preview {
  const [loaded, setLoaded] = useState<Loaded>({ state: "loading" });
  useEffect(() => {
    let current = true;
    loadRenderer().then(
      (render) => {
        if (current) {
          setLoaded({ state: "ready", render });
        }
      },
      () => {
        if (current) {
          setLoaded({ state: "failed" });
        }
      },
    );
    return () => {
      current = false;
    };
  }, []);
  return useMemo((): Preview => {
    if (loaded.state === "ready") {
      return { state: "ready", html: loaded.render(text, imageHosts) };
    }
    return { state: loaded.state };
  }, [loaded, text, imageHosts]);
}
