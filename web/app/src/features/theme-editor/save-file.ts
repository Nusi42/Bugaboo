import { themeFile } from "./themes.ts";

/** The type of a theme file (JSON), and the ending of its name. */
const JSON_TYPE = "application/json";
const JSON_ENDING = ".json";

/** Gives the person a file to keep: the browser saves it, nothing opens in the page. */
function saveAsFile(name: string, text: string): void {
  const address = URL.createObjectURL(new Blob([text], { type: JSON_TYPE }));
  const link = document.createElement("a");
  link.href = address;
  link.download = name;
  link.click();
  URL.revokeObjectURL(address);
}

/** *Download*: the theme as the file an upload takes, named after the theme. */
export async function downloadTheme(theme: string): Promise<void> {
  const file = await themeFile(theme);
  saveAsFile(`${file.name}${JSON_ENDING}`, `${JSON.stringify(file, null, 2)}\n`);
}
