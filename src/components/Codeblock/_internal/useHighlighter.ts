import { useDeepMemo } from "@/hooks/deepMemo";
import { grammarNeedsLoad, highlighter, loadGrammar, loadTheme, themeNeedsLoad } from "@/utils/shiki";
import { Language } from "@/utils/textmate";
import { getLanguageDeps } from "@/utils/textmate/grammars";
import type { TextmateTheme } from "@/utils/textmate/theme";

import { use, useMemo } from "react";
import { type HighlighterCore } from "shiki";


export function useHighlighter(language: Language, theme: TextmateTheme): HighlighterCore {
    const grammarsToLoad = useDeepMemo(grammarNeedsLoad(getLanguageDeps(language)));
    const hasGrammars = grammarsToLoad.length !== 0;

    const themePromise = useMemo(() => {
        if (themeNeedsLoad(theme)) {
            return loadTheme(theme);
        }
    }, [theme]);

    if (themePromise) {
        use(themePromise);
    }

    const grammarsPromise = useMemo(() => {
        if (hasGrammars) {
            return loadGrammar(language);
        }
    }, [hasGrammars, language]);

    if (grammarsPromise) {
        use(grammarsPromise);
    }

    // this will be loaded by the two previous promises
    return highlighter!;
}
