import { makeLazy } from "@/utils/lazy";
import { createOnigurumaEngine } from "@/utils/onigasm";
import { hasGrammar, lazyLoadGrammar } from "@/utils/textmate";

import { type IRawGrammar, Registry } from "vscode-textmate";

export const registry = makeLazy(() => {
    return new Registry({
        onigLib: createOnigurumaEngine(),
        async loadGrammar(scopeName) {
            if (hasGrammar(scopeName)) {
                const content = await lazyLoadGrammar(scopeName);

                return content as IRawGrammar;
            }
            console.warn(`No grammar found for scope name: ${scopeName}`);
            return null;
        },
    });
});
