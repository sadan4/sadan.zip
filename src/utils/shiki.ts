import { languageDisplayNames } from "./textmate/language";
import { TextmateTheme } from "./textmate/theme";
import { getLineNumberColorForTheme, lazyLoadTextmateTheme, type LineNumberColor } from "./textmate/themes";
import { assert } from "./error";
import { type Lazy, makeLazy } from "./lazy";
import { createOnigurumaEngine } from "./oniguruma";
import { getLanguageDeps, hasGrammar, Language, lazyLoadGrammar } from "./textmate";

import { createHighlighterCore, type HighlighterCore } from "shiki/core";
import type { BundledLanguage, BundledTheme, SpecialLanguage } from "shiki/types";

type ShikiLanguage = BundledLanguage | SpecialLanguage;

export const langMap: Record<Language, ShikiLanguage> = Object.freeze({
    [Language.HTML]: "html",
    [Language.JSON]: "json",
    [Language.JAVASCRIPT]: "javascript",
    [Language.JAVASCRIPT_REACT]: "jsx",
    [Language.TYPESCRIPT]: "typescript",
    [Language.TYPESCRIPT_REACT]: "tsx",
    [Language.PLAINTEXT]: "plaintext",
    [Language.UNKNOWN]: "plaintext",
    [Language.CSS]: "css",
});

export const themeMap: Record<TextmateTheme, BundledTheme> = Object.freeze({
    [TextmateTheme.TOKYO_NIGHT]: "tokyo-night",
    [TextmateTheme.ROSE_PINE]: "rose-pine",
    [TextmateTheme.ROSE_PINE_DAWN]: "rose-pine-dawn",
    [TextmateTheme.ROSE_PINE_MOON]: "rose-pine-moon",
    [TextmateTheme.NORD]: "nord",
    [TextmateTheme.CATPPUCCIN_MOCHA]: "catppuccin-mocha",
    [TextmateTheme.CATPPUCCIN_FRAPPE]: "catppuccin-frappe",
    [TextmateTheme.CATPPUCCIN_MACCHIATO]: "catppuccin-macchiato",
    [TextmateTheme.CATPPUCCIN_LATTE]: "catppuccin-latte",
    [TextmateTheme.DRACULA]: "dracula",
});
export let highlighter: HighlighterCore | undefined;

// TODO: make all themes lazy
export const highlighterPromise: Lazy<Promise<HighlighterCore>> = makeLazy(async () => {
    const ret = await createHighlighterCore({
        themes: [],
        engine: createOnigurumaEngine(),
        warnings: true,
    });

    assert(!highlighter);
    highlighter = ret;
    return ret;
});

const loadedThemes = new Set<TextmateTheme>();
const loadedLangs = new Set<Language>();

export function grammarNeedsLoad(languages: Language[]): Language[];
export function grammarNeedsLoad(language: Language): boolean;
export function grammarNeedsLoad(language: Language | Language[]): boolean | Language[] {
    if (Array.isArray(language)) {
        return language.filter((lang) => grammarNeedsLoad(lang));
    }
    return !loadedLangs.has(language) && hasGrammar(language);
}

export function themeNeedsLoad(theme: TextmateTheme): boolean {
    return !loadedThemes.has(theme);
}

export async function loadGrammar(...languages: Language[]): Promise<void> {
    const hl = highlighter ?? await highlighterPromise();

    for (const lang of languages) {
        const grammarsToLoad = grammarNeedsLoad(getLanguageDeps(lang));

        for (const lang of grammarsToLoad) {
            const loadedLang = await lazyLoadGrammar(lang);

            hl.loadLanguageSync(loadedLang);
            loadedLangs.add(lang);
        }
    }
}
export function loadAllGrammars(): Promise<void> {
    return loadGrammar(...Object.keys(languageDisplayNames) as (keyof typeof languageDisplayNames)[]);
}

export function loadAllThemes(): Promise<void> {
    return loadTheme(...Object.values(TextmateTheme).filter((v): v is TextmateTheme => typeof v === "number"));
}

export async function loadTheme(...themes: TextmateTheme[]): Promise<void> {
    const hl = highlighter ?? await highlighterPromise();

    for (const theme of themes) {
        if (themeNeedsLoad(theme)) {
            const loadedTheme = await lazyLoadTextmateTheme(theme);

            hl.loadThemeSync(loadedTheme);
            loadedThemes.add(theme);
        }
    }
}

export function getLineNumberColor(theme: TextmateTheme): LineNumberColor {
    return getLineNumberColorForTheme(highlighter!.getTheme(themeMap[theme]));
}

// load all grammars for SSR
if (import.meta.env.SSR) {
    await Promise.all([loadAllGrammars(), loadAllThemes()]);
}
