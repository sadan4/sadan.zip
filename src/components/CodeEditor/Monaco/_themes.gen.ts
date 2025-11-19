import { dedent } from "../../../utils/string";
import { TextmateTheme } from "../../../utils/textmate/theme";
import { lazyLoadTextmateTheme } from "../../../utils/textmate/themes";

import { convertTheme, type IVSCodeTheme } from "monaco-vscode-textmate-theme-converter";
import type { GeneratorArgs } from "rollup-plugin-generate";

export async function generate({ emitFile }: GeneratorArgs) {
    const typeFile = emitFile({
        content: dedent`
            // This file is generated. Do not edit.
            import type * as monaco from "monaco-editor";

            export type MonacoThemeData = monaco.editor.IStandaloneThemeData;
        `,
        nameHint: "types",
    });

    const ret = [
        dedent`
            // This file is generated. Do not edit.

            import { type MonacoThemeData } from "${typeFile}";

            import { TextmateTheme } from "@/utils/textmate/theme";

            export { type MonacoThemeData };
        `,
    ];

    const themes = Object.values(TextmateTheme).filter((v) => typeof v !== "number") as (keyof typeof TextmateTheme)[];
    const themeValues = themes.map((t) => TextmateTheme[t]);

    for (const theme of themes) {
        const tmTheme = await lazyLoadTextmateTheme(TextmateTheme[theme]);
        const monacoTheme = convertTheme(tmTheme as IVSCodeTheme);

        const themeFile = emitFile({
            content: dedent`
                // This file is generated. Do not edit.
                import { type MonacoThemeData } from "${typeFile}";

                const theme: MonacoThemeData = JSON.parse(${JSON.stringify(JSON.stringify(monacoTheme))});
                export default theme;
            `,
            nameHint: theme,
        });

        ret.push(`export function ${theme}() { return import("${themeFile}").then(({default: d}) => d) }`);
    }

    ret.push(dedent`
        export const loaderMap: Record<TextmateTheme, () => Promise<MonacoThemeData>> = {
            ${
                themeValues
                    .map((v) => {
                        return `[${v}]: ${TextmateTheme[v]},`;
                    })
                    .join("\n")
            }
        };
    `);

    return ret.join("\n");
}
