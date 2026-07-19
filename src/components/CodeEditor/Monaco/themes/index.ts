import { type Monaco, monaco } from "@/utils/monaco";
import { filterObject } from "@/utils/obj";
import { TextmateTheme } from "@/utils/textmate/theme";

import { loaderMap } from "./generated";

import { use } from "react";

const SYM_MONACO_THEME_LIGHT = Symbol("MonacoTheme.LIGHT");
const SYM_MONACO_THEME_DARK = Symbol("MonacoTheme.DARK");
const SYM_MONACO_THEME_HIGH_CONTRAST = Symbol("MonacoTheme.HIGH_CONTRAST");
const SYM_MONACO_THEME_HIGH_CONTRAST_LIGHT = Symbol("MonacoTheme.HIGH_CONTRAST_LIGHT");

export const MonacoTheme = Object.freeze({
    LIGHT: SYM_MONACO_THEME_LIGHT,
    DARK: SYM_MONACO_THEME_DARK,
    HIGH_CONTRAST: SYM_MONACO_THEME_HIGH_CONTRAST,
    HIGH_CONTRAST_LIGHT: SYM_MONACO_THEME_HIGH_CONTRAST_LIGHT,
} as const);

export type BuiltinMonacoTheme = typeof MonacoTheme[keyof typeof MonacoTheme];

export type MonacoTheme = BuiltinMonacoTheme | TextmateTheme;

export const DEFAULT_MONACO_THEME: MonacoTheme = TextmateTheme.TOKYO_NIGHT;

export const AllMonacoThemes = {
    ...MonacoTheme,
    ...filterObject(TextmateTheme, (_, v): v is TextmateTheme => typeof v === "number"),
};

const MonacoThemeStringMap: Record<BuiltinMonacoTheme, Monaco.editor.BuiltinTheme> = {
    [MonacoTheme.LIGHT]: "vs",
    [MonacoTheme.DARK]: "vs-dark",
    [MonacoTheme.HIGH_CONTRAST]: "hc-black",
    [MonacoTheme.HIGH_CONTRAST_LIGHT]: "hc-light",
};

const registeredThemeMap = new Map<TextmateTheme, string>();

function makeLegalThemeName(theme: string): string {
    return theme.replaceAll("_", "-");
}

async function loadLazyTextmateThemeForMonaco(theme: TextmateTheme): Promise<string> {
    const loader = loaderMap[theme];
    const legalThemeName = makeLegalThemeName(TextmateTheme[theme]);
    const data = await loader();

    monaco.editor.defineTheme(legalThemeName, data);

    registeredThemeMap.set(theme, legalThemeName);

    return legalThemeName;
}

export function useMonacoTheme(theme: MonacoTheme): string {
    switch (theme) {
        case TextmateTheme.TOKYO_NIGHT:
        case TextmateTheme.ROSE_PINE:
        case TextmateTheme.ROSE_PINE_DAWN:
        case TextmateTheme.ROSE_PINE_MOON:
        case TextmateTheme.NORD:
        case TextmateTheme.CATPPUCCIN_MOCHA:
        case TextmateTheme.CATPPUCCIN_FRAPPE:
        case TextmateTheme.CATPPUCCIN_MACCHIATO:
        case TextmateTheme.CATPPUCCIN_LATTE:
        case TextmateTheme.GRUVBOX:
        case TextmateTheme.OXOCARBON:
        case TextmateTheme.DRACULA: {
            return registeredThemeMap.get(theme) ?? use(loadLazyTextmateThemeForMonaco(theme));
        }
        case MonacoTheme.DARK:
        case MonacoTheme.LIGHT:
        case MonacoTheme.HIGH_CONTRAST:
        case MonacoTheme.HIGH_CONTRAST_LIGHT: {
            return MonacoThemeStringMap[theme];
        }
    }
}

