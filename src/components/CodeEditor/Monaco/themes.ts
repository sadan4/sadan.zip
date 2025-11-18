import { error } from "@/utils/error";
import { filterObject } from "@/utils/obj";
import { TextmateTheme } from "@/utils/textmate/theme";

import { loaderMap } from "./_themes.gen&gen";

import * as monaco from "monaco-editor";
import { use } from "react";


export const MonacoTheme = Object.freeze({
    LIGHT: Symbol("MonacoTheme.LIGHT"),
    DARK: Symbol("MonacoTheme.DARK"),
    HIGH_CONTRAST: Symbol("MonacoTheme.HIGH_CONTRAST"),
    HIGH_CONTRAST_LIGHT: Symbol("MonacoTheme.HIGH_CONTRAST_LIGHT"),
} as const);

export type BuiltinMonacoTheme = typeof MonacoTheme[keyof typeof MonacoTheme];

export type MonacoTheme = BuiltinMonacoTheme | TextmateTheme;

export const DEFAULT_MONACO_THEME: MonacoTheme = TextmateTheme.TOKYO_NIGHT;

export const AllMonacoThemes = {
    ...MonacoTheme,
    ...filterObject(TextmateTheme, (_, v): v is TextmateTheme => typeof v === "number"),
};

const MonacoThemeStringMap: Record<BuiltinMonacoTheme, monaco.editor.BuiltinTheme> = {
    [MonacoTheme.LIGHT]: "vs",
    [MonacoTheme.DARK]: "vs-dark",
    [MonacoTheme.HIGH_CONTRAST]: "hc-black",
    [MonacoTheme.HIGH_CONTRAST_LIGHT]: "hc-light",
};

const registeredThemeMap: Map<TextmateTheme, string> = new Map();

function makeLegalThemeName(theme: string): string {
    return theme.replaceAll("_", "-");
}

async function loadLazyTextmateThemeForMonaco(theme: TextmateTheme): Promise<string> {
    const loader = loaderMap[theme];
    const legalThemeName = makeLegalThemeName(TextmateTheme[theme]);

    if (!loader) {
        error(`no loader for theme: ${TextmateTheme[theme]}`);
    }

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
        case TextmateTheme.CATPUCCIN_MOCHA:
        case TextmateTheme.CATPUCCIN_FRAPPE:
        case TextmateTheme.CATPUCCIN_MACCHIATO:
        case TextmateTheme.CATPUCCIN_LATTE:
        case TextmateTheme.DRACULA: {
            return registeredThemeMap.get(theme) ?? use(loadLazyTextmateThemeForMonaco(theme));
        }
        case MonacoTheme.DARK:
        case MonacoTheme.LIGHT:
        case MonacoTheme.HIGH_CONTRAST:
        case MonacoTheme.HIGH_CONTRAST_LIGHT: {
            return MonacoThemeStringMap[theme];
        }
        default:
            error(`unknown monaco theme: ${String(theme)} textmateTheme?: ${TextmateTheme[theme as never]}`);
    }
}

