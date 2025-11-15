import { assert } from "@/utils/error";
import { type Lazy, makeLazy } from "@/utils/lazy";
import { hasGrammar, Language, lazyLoadGrammar } from "@/utils/textmate";

import { use } from "react";
import { createHighlighterCore, createOnigurumaEngine } from "react-shiki/core";
import type { HighlighterCore } from "shiki";

let highlighter: HighlighterCore | undefined;

// TODO: make all themes lazy
const highlighterPromise: Lazy<Promise<HighlighterCore>> = makeLazy(async () => {
    const ret = await createHighlighterCore({
        themes: [import("@shikijs/themes/tokyo-night")],
        langs: [],
        engine: createOnigurumaEngine(import("shiki/wasm")),
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

function needsLoad(language: Language): boolean {
    return !loadedLangs.has(language) && hasGrammar(language);
}

export function useHighlighter(language: Language): HighlighterCore {
    if (needsLoad(language)) {
        // preload it
        lazyLoadGrammar(language);
    }

    const hl = highlighter ?? use(highlighterPromise());

    if (needsLoad(language)) {
        use(lazyLoadGrammar(language).then((lang) => {
            hl.loadLanguageSync(lang);
            loadedLangs.add(language);
        }));
    }
    return hl;
}
