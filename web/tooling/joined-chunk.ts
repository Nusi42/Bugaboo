/** What the build knows of a module: what it imports statically and with `import()`, and whether it is an entry. */
export interface ModuleLinks {
  readonly importedIds: readonly string[];
  readonly dynamicallyImportedIds: readonly string[];
  readonly isEntry: boolean;
}

/** The file whose `import()` calls are the features every signed-in first page waits for (`routes/joined.ts`). */
export const JOINED_MODULE = /[\\/]app[\\/]src[\\/]routes[\\/]joined\.ts$/u;

/**
 * The code of the app and its texts, which the joined file takes: the features, their shared code and their message
 * parts. The components and the libraries keep their own files, which other pages share.
 */
const APP_CODE = /[\\/]web[\\/](app[\\/]src|packages[\\/]messages)[\\/]/u;

/** The name of the file the joined features and what they need are built into (a few files, by `SPLIT`). */
export const JOINED_CHUNK = "joined";

/**
 * The texts of the joined features stand in a file of their own beside it, so neither file grows past what the build
 * warns about; texts import nothing of the features, so the files load in any order.
 */
export const JOINED_TEXTS_CHUNK = "joined-texts";

/**
 * The largest of the joined features stand in files of their own for the same reason, each with the shared code of the
 * app beside them: the issues, and the search with the relations.
 */
const SPLIT: readonly (readonly [RegExp, string])[] = [
  [/[\\/]app[\\/]src[\\/]features[\\/]issues[\\/]/u, "joined-issues"],
  [/[\\/]app[\\/]src[\\/]features[\\/](search|relations)[\\/]/u, "joined-search"],
  [/[\\/]app[\\/]src[\\/]shared[\\/]/u, "joined-shared"],
];

/** The message parts of the browser app. */
const TEXTS = /[\\/]packages[\\/]messages[\\/]/u;

/** Every module reached from `roots` by static imports, the roots too. */
function staticallyReached(
  roots: readonly string[],
  linksOf: (id: string) => ModuleLinks | null,
): Set<string> {
  const reached = new Set<string>();
  const waiting = [...roots];
  for (let id = waiting.pop(); id !== undefined; id = waiting.pop()) {
    if (!reached.has(id)) {
      reached.add(id);
      waiting.push(...(linksOf(id)?.importedIds ?? []));
    }
  }
  return reached;
}

/**
 * The modules of the joined file: those the joined features need, by static imports, that the first file of the page
 * does not hold already (what an entry reaches statically stays there, so the first page's own file does not grow).
 */
export function joinedModules(
  ids: readonly string[],
  linksOf: (id: string) => ModuleLinks | null,
): Set<string> {
  const joined = ids.find((id) => JOINED_MODULE.test(id));
  if (joined === undefined) {
    return new Set();
  }
  const entries = ids.filter((id) => linksOf(id)?.isEntry === true);
  const initial = staticallyReached(entries, linksOf);
  const needed = staticallyReached(linksOf(joined)?.dynamicallyImportedIds ?? [], linksOf);
  return new Set([...needed].filter((id) => !initial.has(id) && APP_CODE.test(id)));
}

/**
 * The group of the build that puts the joined features into one file (UX §6.4, PERF-04): before the first page of
 * somebody who is signed in, the app waits for them all anyway, and a browser that opens a connection for each file
 * (WebKit on Windows) took a fifth of a second for each of the dozens of small files they were split into. Code that
 * other pages load later and the first file of the page are not touched.
 */
export function joinedGroup(): {
  readonly includeDependenciesRecursively: false;
  readonly name: (
    id: string,
    ctx: { readonly getModuleInfo: (id: string) => ModuleLinks | null },
  ) => string | null;
} {
  const known = new Map<string, Set<string>>();
  return {
    // Only the modules found here: by default the build would pull their dependencies in too, among them what the
    // first file of the page holds, which would then load the joined file before anything.
    includeDependenciesRecursively: false,
    name: (id, ctx) => {
      let joined = known.get("all");
      if (joined === undefined) {
        const ids = [...allModules((each) => ctx.getModuleInfo(each), id)];
        joined = joinedModules(ids, (each) => ctx.getModuleInfo(each));
        known.set("all", joined);
      }
      if (!joined.has(id)) {
        return null;
      }
      return joinedChunkOf(id);
    },
  };
}

/** Which joined file a module of the joined features stands in. */
function joinedChunkOf(id: string): string {
  if (TEXTS.test(id)) {
    return JOINED_TEXTS_CHUNK;
  }
  for (const [part, name] of SPLIT) {
    if (part.test(id)) {
      return name;
    }
  }
  return JOINED_CHUNK;
}

/** Every module the build knows, found from one of them through its imports and importers. */
function allModules(
  linksOf: (id: string) =>
    | (ModuleLinks & {
        readonly importers?: readonly string[];
        readonly dynamicImporters?: readonly string[];
      })
    | null,
  start: string,
): Set<string> {
  const seen = new Set<string>();
  const waiting = [start];
  for (let id = waiting.pop(); id !== undefined; id = waiting.pop()) {
    if (!seen.has(id)) {
      seen.add(id);
      const links = linksOf(id);
      waiting.push(
        ...(links?.importedIds ?? []),
        ...(links?.dynamicallyImportedIds ?? []),
        ...(links?.importers ?? []),
        ...(links?.dynamicImporters ?? []),
      );
    }
  }
  return seen;
}
