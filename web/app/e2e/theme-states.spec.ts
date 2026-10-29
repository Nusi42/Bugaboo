import { expect, test } from "./support/fixtures.ts";
import { webIssues } from "./support/issues.ts";
import { layoutOf } from "./support/layout.ts";
import { focusRowByKeys, tickIssues } from "./support/list-pictures.ts";
import { openList, prepareLook, rowOf } from "./support/list-look.ts";
import { inEveryLook, resolved } from "./support/styles.ts";

// M5 exit, package T (L24, L26): the edge of High is its own clear orange, a pointed-at row has a line
// of its own color, and the keyboard focus on a row is two rings with a gap between them.

test.use({ preparation: { run: prepareLook } });
test.describe.configure({ mode: "serial" });

test("ux_7_2 the edge of Critical is the red and the edge of High the orange of the theme, in every theme", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "the rows of a phone are drawn by the same rules");
  await openList(page);
  await inEveryLook(page, async (look) => {
    await expect(rowOf(page, webIssues.login), look).toHaveCSS(
      "border-inline-start-color",
      await resolved(page, "border-top-color", "var(--danger)"),
    );
    await expect(rowOf(page, webIssues.passkey), look).toHaveCSS(
      "border-inline-start-color",
      await resolved(page, "border-top-color", "var(--edge-high)"),
    );
  });
});

test("ux_7_2 a row that is pointed at has the hover line of the theme, in every theme", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "a phone has no pointer");
  await openList(page);
  const row = rowOf(page, webIssues.typo);
  await row.hover();
  await inEveryLook(page, async (look) => {
    await row.hover();
    const line = await resolved(page, "border-top-color", "var(--hover-line)");
    expect(await row.evaluate((node) => getComputedStyle(node).boxShadow), look).toContain(line);
  });
});

test("ux_7_2 the keyboard focus on a row is two rings with a gap, in every theme", async ({
  page,
}) => {
  test.skip(layoutOf(page) === "phone", "a phone has no keyboard");
  await openList(page);
  await tickIssues(page, ["WEB-1"]);
  const row = rowOf(page, webIssues.typo);
  await focusRowByKeys(page, row);
  await inEveryLook(page, async (look) => {
    const shadow = await row.evaluate((node) => getComputedStyle(node).boxShadow);
    const gap = await resolved(page, "border-top-color", "var(--bg-raised)");
    const accent = await resolved(page, "border-top-color", "var(--accent)");
    const text = await resolved(page, "border-top-color", "var(--fg)");
    expect(shadow, look).toContain(accent);
    expect(shadow, look).toContain(gap);
    expect(shadow, look).toContain(text);
    // The rings come from the outside in: the accent, the gap, the text color.
    expect(shadow.indexOf(accent), look).toBeLessThan(shadow.indexOf(gap));
    expect(shadow.indexOf(gap), look).toBeLessThan(shadow.indexOf(text));
  });
});
