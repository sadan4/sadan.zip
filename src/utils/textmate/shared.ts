import { languagesWithGrammars } from "./_grammars.gen&gen";
import type { Language } from "./language";

export function hasGrammar(language: string | Language): language is Language {
    return languagesWithGrammars.has(language as Language);
}
