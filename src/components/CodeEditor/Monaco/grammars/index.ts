import { error } from "@/utils/error";
import { makeLazy } from "@/utils/lazy";

import { Registry } from "monaco-textmate";

export const registry = makeLazy(() => {
    return new Registry({
        async getGrammarDefinition(scopeName) {
            if (scopeName.endsWith("tsx")) {
                const tsx = await import("@shikijs/langs/tsx");

                return {
                    format: "json",
                    content: tsx.default[0],
                };
            }
            if (scopeName.endsWith("ts")) {
                const ts = await import("@shikijs/langs/typescript");

                return {
                    format: "json",
                    content: ts.default[0],
                };
            }
            if (scopeName.endsWith("js")) {
                const js = await import("@shikijs/langs/javascript");

                return {
                    format: "json",
                    content: js.default[0],
                };
            }
            if (scopeName.endsWith("jsx")) {
                const jsx = await import("@shikijs/langs/jsx");

                return {
                    format: "json",
                    content: jsx.default[0],
                };
            }
            if (scopeName.endsWith("json")) {
                const json = await import("@shikijs/langs/json");

                return {
                    format: "json",
                    content: json.default[0],
                };
            }
            if (scopeName.endsWith("html")) {
                const html = await import("@shikijs/langs/html");

                return {
                    format: "json",
                    content: html.default[0],
                };
            }
            error(`missing textmate grammar for file ${scopeName}`);
        },
    });
});
