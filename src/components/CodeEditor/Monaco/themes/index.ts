import { unreachable } from "@/utils/error";
import { type Lazy, makeLazy } from "@/utils/lazy";

import * as monaco from "monaco-editor";
import { convertTheme, type IVSCodeTheme } from "monaco-vscode-textmate-theme-converter";
import { use } from "react";

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

type VSCodeTheme = {
    default: (IVSCodeTheme | {
        schema?: null | undefined;
    }) & { name: string; };
};

function lazyTheme(getter: () => Promise<VSCodeTheme>): Lazy<Promise<string>> {
    return makeLazy(async () => {
        const { default: { name, ...json } } = await getter();
        const monacoTheme = convertTheme(json as IVSCodeTheme);

        monaco.editor.defineTheme(name, monacoTheme);
        return name;
    });
}

const TokyoNight = lazyTheme(() => import("./tokyoNight.json"));

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
            return use(TokyoNight());
        default:
            unreachable();
    }
}
