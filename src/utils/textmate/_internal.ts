import { Language } from "./language";

import type { LanguageRegistration } from "shiki";

export const grammars: Record<Language, null | (() => Promise<LanguageRegistration>)> = {
    async [Language.JSON]() {
        const json = await import("@shikijs/langs/json");

        return json.default[0];
    },
    async [Language.JAVASCRIPT]() {
        const js = await import("@shikijs/langs/javascript");

        return js.default[0];
    },
    async [Language.TYPESCRIPT]() {
        const ts = await import("@shikijs/langs/typescript");

        return ts.default[0];
    },
    async [Language.TYPESCRIPT_REACT]() {
        const tsx = await import("@shikijs/langs/tsx");

        return tsx.default[0];
    },
    async [Language.JAVASCRIPT_REACT]() {
        const jsx = await import("@shikijs/langs/jsx");

        return jsx.default[0];
    },
    async [Language.HTML]() {
        const html = await import("@shikijs/langs/html");

        return html.default[0];
    },
    [Language.PLAINTEXT]: null,
    [Language.UNKNOWN]: null,
};
