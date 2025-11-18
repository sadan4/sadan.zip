import { TextmateTheme } from "./theme";

import * as shiki from "shiki";

export type TMTheme = shiki.ThemeRegistration;

const textmateThemes: Record<TextmateTheme, () => Promise<TMTheme>> = {
    async [TextmateTheme.ROSE_PINE](): Promise<TMTheme> {
        const theme = await import("@shikijs/themes/rose-pine");

        return theme.default;
    },
    async [TextmateTheme.ROSE_PINE_DAWN](): Promise<TMTheme> {
        const theme = await import("@shikijs/themes/rose-pine-dawn");

        return theme.default;
    },
    async [TextmateTheme.ROSE_PINE_MOON](): Promise<TMTheme> {
        const theme = await import("@shikijs/themes/rose-pine-moon");

        return theme.default;
    },
    async [TextmateTheme.NORD](): Promise<TMTheme> {
        const theme = await import("@shikijs/themes/nord");

        return theme.default;
    },
    async [TextmateTheme.CATPUCCIN_MOCHA](): Promise<TMTheme> {
        const theme = await import("@shikijs/themes/catppuccin-mocha");

        return theme.default;
    },
    async [TextmateTheme.CATPUCCIN_FRAPPE](): Promise<TMTheme> {
        const theme = await import("@shikijs/themes/catppuccin-frappe");

        return theme.default;
    },
    async [TextmateTheme.CATPUCCIN_MACCHIATO](): Promise<TMTheme> {
        const theme = await import("@shikijs/themes/catppuccin-macchiato");

        return theme.default;
    },
    async [TextmateTheme.CATPUCCIN_LATTE](): Promise<TMTheme> {
        const theme = await import("@shikijs/themes/catppuccin-latte");

        return theme.default;
    },
    async [TextmateTheme.DRACULA](): Promise<TMTheme> {
        const theme = await import("@shikijs/themes/dracula");

        return theme.default;
    },
    async [TextmateTheme.TOKYO_NIGHT](): Promise<TMTheme> {
        const theme = await import("@shikijs/themes/tokyo-night");

        return theme.default;
    },
};

export function lazyLoadTextmateTheme(theme: TextmateTheme): Promise<TMTheme> {
    return textmateThemes[theme]();
}
