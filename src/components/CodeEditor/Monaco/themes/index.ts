import { unreachable } from "@/utils/error";
import { makeLazy } from "@/utils/lazy";

import * as themeGen from "./_themes.gen?gen";

import * as monaco from "monaco-editor";

export const enum MonacoTheme {
    LIGHT,
    DARK,
    HIGH_CONTRAST,
    HIGH_CONTRAST_LIGHT,
    TOKYO_NIGHT,
    TOKYO_NIGHT_STORM,
    TOKYO_NIGHT_LIGHT,
}

export const DEFAULT_MONACO_THEME = MonacoTheme.TOKYO_NIGHT;

function lazyTheme(name: string, theme: monaco.editor.IStandaloneThemeData) {
    return makeLazy(() => {
        monaco.editor.defineTheme(name, theme);
        return name;
    });
}

const TokyoNight = lazyTheme("TokyoNight", themeGen.TokyoNight);

export function useThemeString(theme: MonacoTheme): string {
    switch (theme) {
        case MonacoTheme.LIGHT:
            return "vs";
        case MonacoTheme.DARK:
            return "vs-dark";
        case MonacoTheme.HIGH_CONTRAST:
            return "hc-black";
        case MonacoTheme.HIGH_CONTRAST_LIGHT:
            return "hc-light";
        case MonacoTheme.TOKYO_NIGHT:
            return TokyoNight();
        default:
            unreachable();
    }
}
