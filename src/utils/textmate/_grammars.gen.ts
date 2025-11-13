import { grammars } from "./_internal";
import { languagesWithGrammars } from "./grammars";
import { dedent } from "../string";

import type { GeneratorArgs } from "rollup-plugin-generate";

export async function generate(_: GeneratorArgs) {
    const langs = languagesWithGrammars();

    const output = [
        dedent`
            // This file is generated. Do not edit.,

            import { makeLazy, type Lazy } from "@/utils/lazy";

            import * as shiki from "shiki";

            export type LazyLang = Lazy<shiki.LanguageRegistration>;
        `,
    ];

    for (const lang of langs) {
        const def = await grammars[lang]!();

        output.push(dedent`
            export const ${lang}: LazyLang = /* @__PURE__ */ makeLazy(() => {
                const lang = ${JSON.stringify(def)};
                return lang;
            });
        `);
    }
    return output.join("\n");
}
