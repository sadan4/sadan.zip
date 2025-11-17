import { error } from "@/utils/error";
import { makeLazy } from "@/utils/lazy";
import { hasGrammar } from "@/utils/textmate";

import guh from "./test.json";

import { Registry } from "monaco-textmate";

export const registry = makeLazy(() => {
    return new Registry({
        async getGrammarDefinition(scopeName) {
            if (hasGrammar(scopeName)) {
                // const [content] = await lazyLoadGrammar(scopeName);

                return {
                    format: "json",
                    content: guh,
                };
            }
            error(`No grammar found for scope name: ${scopeName}`);
        },
    });
});
