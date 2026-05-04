import { languagesWithGrammars } from "./grammars/generated";
import type { Language } from "./language";

export function hasGrammar(language: string | Language): language is Language {
    return languagesWithGrammars.has(language as Language);
}
