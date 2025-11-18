import { assert } from "@/utils/error";
import { type Lazy, makeLazy } from "@/utils/lazy";
import { createOnigurumaEngine } from "@/utils/oniguruma";
import { hasGrammar, Language, lazyLoadGrammar } from "@/utils/textmate";
import type { LazyLang } from "@/utils/textmate/_grammars.gen&gen";
import { getLanguageDeps } from "@/utils/textmate/grammars";

import { use } from "react";
import { createHighlighterCore } from "react-shiki/core";
import type { HighlighterCore } from "shiki";

let highlighter: HighlighterCore | undefined;

// TODO: make all themes lazy
const highlighterPromise: Lazy<Promise<HighlighterCore>> = makeLazy(async () => {
    const ret = await createHighlighterCore({
        themes: [import("@shikijs/themes/tokyo-night")],
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

// const loadedThemes = new Set<Language>();
const loadedLangs = new Set<Language>();

function needsLoad(languages: Language[]): Language[];
function needsLoad(language: Language): boolean;
function needsLoad(language: Language | Language[]): boolean | Language[] {
    if (Array.isArray(language)) {
        return language.filter((lang) => needsLoad(lang));
    }
    return !loadedLangs.has(language) && hasGrammar(language);
}

export function useHighlighter(language: Language): HighlighterCore {
    const toLoad = needsLoad(getLanguageDeps(language));
    let promises: readonly [Language, Promise<LazyLang>][] = [];

    if (toLoad.length) {
        // preload it
        promises = toLoad.map((lang) => [lang, lazyLoadGrammar(lang)] as const);
    }

    const hl = highlighter ?? use(highlighterPromise());

    if (promises.length) {
        use(Promise.all(promises.map(([lang, p]) => p.then((loadedLang) => {
            hl.loadLanguageSync(loadedLang);
            loadedLangs.add(lang);
        }))));
    }
    return hl;
}
