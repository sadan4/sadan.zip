import { html, javascript, javascriptreact, json, type LazyLang, typescript, typescriptreact } from "./_grammars.gen&gen";
import { Language } from ".";
import { error } from "../error";

export function lazyLoadGrammar(language: Language): Promise<LazyLang> {
    switch (language) {
        case Language.JSON: {
            return json();
        }
        case Language.TYPESCRIPT: {
            return typescript();
        }
        case Language.JAVASCRIPT: {
            return javascript();
        }
        case Language.TYPESCRIPT_REACT: {
            return typescriptreact();
        }
        case Language.JAVASCRIPT_REACT: {
            return javascriptreact();
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
