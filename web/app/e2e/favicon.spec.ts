import { expect, test } from "./support/fixtures.ts";

// The icon of the app: a white ghost on a square in the accent color, made by `cargo xtask gen theme` from the
// theme file. The page names it, and the server serves it at the root, like the rest of the built app.

/** The icons the page names, by how it names them, with the kind of file each is. */
const icons = [
  { link: 'link[rel="icon"][type="image/svg+xml"]', path: "/favicon.svg", type: "image/svg+xml" },
  { link: 'link[rel="icon"][type="image/png"]', path: "/favicon-32.png", type: "image/png" },
  { link: 'link[rel="apple-touch-icon"]', path: "/apple-touch-icon.png", type: "image/png" },
] as const;

test("ux_the page names the icon of the app, and the server serves it from the root", async ({
  page,
}) => {
  await page.goto("/");
  await Promise.all(
    icons.map(async (icon) => {
      await expect(page.locator(`head ${icon.link}`)).toHaveAttribute("href", icon.path);
      const answer = await page.request.get(icon.path);
      expect(answer.status(), icon.path).toBe(200);
      expect(answer.headers()["content-type"], icon.path).toBe(icon.type);
    }),
  );
});
