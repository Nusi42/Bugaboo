import { t } from "@bugaboo/messages";
import { Link, partOf, part } from "@bugaboo/ui";

import type { Attempt } from "./agent-work.ts";

type Result = Attempt["results"][number];

/** The links first and then the checks, the same on every screen. */
function inOrder(results: readonly Result[]): readonly Result[] {
  return [
    ...results.filter((result) => result.kind !== "check"),
    ...results.filter((result) => result.kind === "check"),
  ];
}

/** What tells a result from the others of its attempt. */
function keyOf(result: Result, at: number): string {
  return [at, result.kind, result.label].join("/");
}

/** One result in words: a link the agent made, or a check with its outcome. */
function ResultLine({ result }: { readonly result: Result }) {
  if (result.kind === "check") {
    const failed = result.outcome === "failed";
    return (
      <li {...partOf(part.agentWorkResult, { variants: [failed && "failed"] })}>
        {t(failed ? "agent-work.result-check-failed" : "agent-work.result-check-passed", {
          label: result.label,
        })}
      </li>
    );
  }
  // The server sends `link` only for a web address (`is_web_address`); anything else is text.
  const { link } = result;
  return (
    <li {...partOf(part.agentWorkResult)}>
      {link === null || link === undefined || link === "" ? (
        result.label
      ) : (
        <Link href={link} external>
          {result.label}
        </Link>
      )}
    </li>
  );
}

/** What an attempt attached: links and checks. Bugaboo does not run or verify the checks, and says so. */
export function Results({ attempt }: { readonly attempt: Attempt }) {
  if (attempt.results.length === 0) {
    return null;
  }
  const checks = attempt.results.some((result) => result.kind === "check");
  return (
    <div {...partOf(part.agentWorkSection)}>
      <ul {...partOf(part.agentWorkResults)} aria-label={t("agent-work.results")}>
        {inOrder(attempt.results).map((result, at) => (
          <ResultLine key={keyOf(result, at)} result={result} />
        ))}
      </ul>
      {checks ? <p {...partOf(part.muted)}>{t("agent-work.checks-note")}</p> : null}
    </div>
  );
}
