import type { LazyLang } from "./_grammars.gen&gen";
import { grammars } from "./_internal";
import { Language } from ".";
import { error } from "../error";
import { makeLazy } from "../lazy";

export function lazyLoadGrammar(language: Language): Promise<LazyLang> {
    switch (language) {
        case Language.JSON: {
            return import("./_grammars.gen&gen")
                .then((mod) => mod.json);
        }
        case Language.TYPESCRIPT: {
            return import("./_grammars.gen&gen")
                .then((mod) => mod.typescript);
        }
        case Language.JAVASCRIPT: {
            return import("./_grammars.gen&gen")
                .then((mod) => mod.javascript);
        }
        case Language.TYPESCRIPT_REACT: {
            return import("./_grammars.gen&gen")
                .then((mod) => mod.typescriptreact);
        }
        case Language.JAVASCRIPT_REACT: {
            return import("./_grammars.gen&gen")
                .then((mod) => mod.javascriptreact);
        }
        case Language.HTML: {
            return import("./_grammars.gen&gen")
                .then((mod) => mod.html);
        }

        case Language.PLAINTEXT:
        case Language.UNKNOWN:
        default:
            error(`No grammar available for language: ${language}`);
    }
}

export const languagesWithGrammars = makeLazy(() => {
    return new Set(Object.entries(grammars)
        .filter(([, loader]) => loader !== null)
        .map(([lang]) => lang as Language));
});

export function hasGrammar(language: string | Language): language is Language {
    return languagesWithGrammars().has(language as Language);
}
