import { error } from "@/utils/error";
import { makeLazy } from "@/utils/lazy";
import { hasGrammar, lazyLoadGrammar } from "@/utils/textmate";

import { Registry } from "monaco-textmate";

export const registry = makeLazy(() => {
    return new Registry({
        async getGrammarDefinition(scopeName) {
            if (hasGrammar(scopeName)) {
                const content = await lazyLoadGrammar(scopeName);

                return {
                    format: "json",
                    content,
                };
            }
            error(`No grammar found for scope name: ${scopeName}`);
        },
    });
});

export const textmateLanguageMap = new Map<string, string>();
