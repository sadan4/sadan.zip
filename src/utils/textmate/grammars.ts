import { html, js, json, jsx, type LazyLang, ts, tsx } from "./_grammars.gen&gen";
import { Language } from ".";
import { error } from "../error";

export function lazyLoadGrammar(language: Language): Promise<LazyLang> {
    switch (language) {
        case Language.JSON: {
            return json();
        }
        case Language.TYPESCRIPT: {
            return ts();
        }
        case Language.JAVASCRIPT: {
            return js();
        }
        case Language.TYPESCRIPT_REACT: {
            return tsx();
        }
        case Language.JAVASCRIPT_REACT: {
            return jsx();
        }
        case Language.HTML: {
            return html();
        }

        case Language.PLAINTEXT:
        case Language.UNKNOWN:
        default:
            error(`No grammar available for language: ${language}`);
    }
}
