import { css, html, js, json, jsx, type LazyLang, ts, tsx } from "./_grammars.gen&gen";
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
        case Language.CSS: {
            return css();
        }

        case Language.PLAINTEXT:
        case Language.UNKNOWN:
            error(`No grammar available for language: ${language}`);
    }
}

export function getLanguageDeps(language: Language): Language[] {
    switch (language) {
        case Language.HTML:
            return [
                ...getLanguageDeps(Language.CSS),
                ...getLanguageDeps(Language.JAVASCRIPT),
                Language.HTML,
            ];
        case Language.PLAINTEXT:
        case Language.UNKNOWN:
        case Language.JSON:
        case Language.TYPESCRIPT:
        case Language.JAVASCRIPT:
        case Language.TYPESCRIPT_REACT:
        case Language.JAVASCRIPT_REACT:
        case Language.CSS:
            return [language];
    }
}
