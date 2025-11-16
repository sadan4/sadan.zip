import { Language } from "./language";
import { dedent } from "../string";

import type { GeneratorArgs, GeneratorExportModuleSideEffects } from "rollup-plugin-generate";
import type { LanguageRegistration } from "shiki";

const grammars: Record<Language, null | (() => Promise<LanguageRegistration[]>)> = {
    async [Language.JSON]() {
        const json = await import("@shikijs/langs/json");

        return json.default;
    },
    async [Language.JAVASCRIPT]() {
        const js = await import("@shikijs/langs/javascript");

        return js.default;
    },
    async [Language.TYPESCRIPT]() {
        const ts = await import("@shikijs/langs/typescript");

        return ts.default;
    },
    async [Language.TYPESCRIPT_REACT]() {
        const tsx = await import("@shikijs/langs/tsx");

        return tsx.default;
    },
    async [Language.JAVASCRIPT_REACT]() {
        const jsx = await import("@shikijs/langs/jsx");

        return jsx.default;
    },
    async [Language.HTML]() {
        const html = await import("@shikijs/langs/html");

        return html.default;
    },
    [Language.PLAINTEXT]: null,
    [Language.UNKNOWN]: null,
};

const languagesWithGrammars = new Set(Object.entries(grammars)
    .filter(([, loader]) => loader !== null)
    .map(([lang]) => lang as Language));

export const moduleSideEffects: GeneratorExportModuleSideEffects = false;

export async function generate({ emitFile }: GeneratorArgs) {
    const langs = languagesWithGrammars;

    const typeFile = emitFile({
        extension: "ts",
        content: dedent`
            // This file is generated. Do not edit.

            import * as shiki from "shiki";

            export type LazyLang = shiki.LanguageRegistration[];
        `,
        nameHint: "types",
        hasSideEffects: false,
    });

    const output = [
        dedent`
            // This file is generated. Do not edit.

            import type { LazyLang } from "${typeFile}";

            export type { LazyLang };

            import type { Language } from "./language";

            export const languagesWithGrammars: Set<Language> = new Set(${JSON.stringify(Array.from(languagesWithGrammars.values()))});
        `,
    ];

    for (const lang of langs) {
        const def = await grammars[lang]!();

        const ref = emitFile({
            extension: "ts",
            content: dedent`
                // This file is generated. Do not edit.

                import type { LazyLang } from "${typeFile}";
                const lang: LazyLang = /* @__PURE__ */ JSON.parse(${JSON.stringify(JSON.stringify(def))});
                export default lang;
            `,
            nameHint: lang,
        });

        const [ident] = lang.match(/[^.]+$/)!;

        output.push(`export function ${ident}(): Promise<LazyLang> { return import("${ref}").then(({default: d}) => d); }`);
    }
    return output.join("\n");
}
