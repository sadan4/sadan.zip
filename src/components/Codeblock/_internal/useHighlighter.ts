import { assert } from "@/utils/error";
import { type Lazy, makeLazy } from "@/utils/lazy";

import { useState } from "react";
import { createHighlighterCore, createOnigurumaEngine } from "react-shiki/core";
import type { HighlighterCore } from "shiki";

let highlighter: HighlighterCore | undefined;

const highlighterPromise: Lazy<Promise<HighlighterCore>> = makeLazy(async () => {
    const ret = await createHighlighterCore({
        themes: [import("@shikijs/themes/tokyo-night")],
        langs: [import("@shikijs/langs/tsx"), import("@shikijs/langs/html")],
        engine: createOnigurumaEngine(import("shiki/wasm")),
    });

    assert(!highlighter);
    highlighter = ret;
    return ret;
});

export function preloadHighlighter() {
    highlighterPromise();
}

export function useHighlighter(): HighlighterCore | undefined {
    const [hl, setHl] = useState<HighlighterCore | undefined>(highlighter);

    if (!hl) {
        highlighterPromise().then(setHl);
    }

    return hl;
}
