import { DEFAULT_THEME, TextmateTheme } from "@/utils/textmate/theme";
import { createServerFn } from "@tanstack/react-start";
import { deleteCookie, getCookie, setCookie } from "@tanstack/react-start/server";

import { createContext } from "react";
import { z } from "zod";

export const ThemeContext = createContext<ThemeContext>(null!);

ThemeContext.displayName = "ThemeContext";

export interface ThemeContext {
    theme: TextmateTheme;
    updateTheme(newTheme: TextmateTheme): void;
}

const COOKIE_NAME = "theme";
let themeCache = DEFAULT_THEME;

export function getCachedTheme(): TextmateTheme {
    return themeCache;
}

export function setCachedTheme(t: TextmateTheme) {
    themeCache = t;
}

const themeSchema = z.enum(TextmateTheme);

export const getThemeServerFn = createServerFn()
    .handler((): TextmateTheme => {
        const cookie = getCookie(COOKIE_NAME);
        let raw = cookie ?? DEFAULT_THEME;

        // cookie is a string, enum parses as a number
        if (typeof raw === "string" && !isNaN(raw as any)) {
            raw = +raw;
        }

        const parsed = themeSchema.safeParse(raw);

        if (parsed.success) {
            return parsed.data;
        }

        deleteCookie(COOKIE_NAME);
        return DEFAULT_THEME;
    });

export const setThemeServerFn = createServerFn()
    .validator(themeSchema)
    .handler(({ data }) => {
        setCookie(COOKIE_NAME, `${data}`);
    });
