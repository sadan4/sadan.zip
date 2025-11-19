import { assert } from "@/utils/error";
import { type Lazy, makeLazy } from "@/utils/lazy";
import { createOnigurumaEngine } from "@/utils/oniguruma";
import { hasGrammar, Language, lazyLoadGrammar } from "@/utils/textmate";
import type { LazyLang } from "@/utils/textmate/_grammars.gen&gen";
import { getLanguageDeps } from "@/utils/textmate/grammars";
import type { TextmateTheme } from "@/utils/textmate/theme";
import { lazyLoadTextmateTheme, type TMTheme } from "@/utils/textmate/themes";

import { use } from "react";
import { createHighlighterCore } from "react-shiki/core";
import type { HighlighterCore } from "shiki";

let highlighter: HighlighterCore | undefined;

// TODO: make all themes lazy
const highlighterPromise: Lazy<Promise<HighlighterCore>> = makeLazy(async () => {
    const ret = await createHighlighterCore({
        themes: [],
        engine: createOnigurumaEngine(),
        warnings: true,
    });

    assert(!highlighter);
    highlighter = ret;
    return ret;
});

export function preloadHighlighter() {
    highlighterPromise();
}

const loadedThemes = new Set<TextmateTheme>();
const loadedLangs = new Set<Language>();

function grammarNeedsLoad(languages: Language[]): Language[];
function grammarNeedsLoad(language: Language): boolean;
function grammarNeedsLoad(language: Language | Language[]): boolean | Language[] {
    if (Array.isArray(language)) {
        return language.filter((lang) => grammarNeedsLoad(lang));
    }
    return !loadedLangs.has(language) && hasGrammar(language);
}

function themeNeedsLoad(theme: TextmateTheme): boolean {
    return !loadedThemes.has(theme);
}

export function useHighlighter(language: Language, theme: TextmateTheme): HighlighterCore {
    const toLoad = grammarNeedsLoad(getLanguageDeps(language));
    let grammarPromises: readonly [Language, Promise<LazyLang>][] = [];
    let themePromise: Promise<TMTheme> | null = null;

    if (toLoad.length) {
        // preload it
        grammarPromises = toLoad.map((lang) => [lang, lazyLoadGrammar(lang)] as const);
    }

    if (themeNeedsLoad(theme)) {
        themePromise = lazyLoadTextmateTheme(theme);
    }

    const hl = highlighter ?? use(highlighterPromise());

    if (themePromise) {
        use(themePromise.then((tmTheme) => {
            hl.loadThemeSync(tmTheme);
            loadedThemes.add(theme);
        }));
    }
    if (grammarPromises.length) {
        use(Promise.all(grammarPromises.map(([lang, p]) => p.then((loadedLang) => {
            hl.loadLanguageSync(loadedLang);
            loadedLangs.add(lang);
        }))));
    }
    return hl;
}
