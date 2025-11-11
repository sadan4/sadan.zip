import { parse } from "jsonc-parser/lib/esm/main.js";
import { convertTheme } from "monaco-vscode-textmate-theme-converter/lib/cjs";
import { readdir, readFile } from "node:fs/promises";
import { extname, join } from "node:path";
import type { GeneratorArgs } from "rollup-plugin-generate";

function camelToPascal(word: string) {
    return `${word.charAt(0).toUpperCase()}${word.slice(1)}`;
}

export async function generate({ watch, dirname }: GeneratorArgs) {
    const themes = (await readdir(dirname)).filter((file) => extname(file) === ".json");
    const output: string[] = [];

    output.push("// This file is auto-generated. Do not edit.");
    output.push("");
    output.push(`import * as monaco from "monaco-editor";`);

    const themeType = "monaco.editor.IStandaloneThemeData";

    for (const theme of themes) {
        const ident = camelToPascal(theme.replace(extname(theme), ""));
        const themeJson = parse(await readFile(join(dirname, theme), "utf-8"));
        const monacoTheme = convertTheme(themeJson);

        watch(join(dirname, theme));

        output.push(`export const ${ident}: ${themeType} = ${JSON.stringify(monacoTheme)};`);
        output.push("");
    }
    return output.join("\n");
}
