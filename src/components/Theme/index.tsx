import { themeStore } from "@/stores/ThemeStore";
import { getCachedThemeLoader } from "@/styles/themes";
import { cn } from "@/utils/cn";
import { TextmateTheme } from "@/utils/textmate/theme";

import { ThemeContext } from "./context";

import { type PropsWithChildren, use, useEffect, useMemo } from "react";

export function RootThemeProvider({ children }: PropsWithChildren) {
    const globalTheme = themeStore((x) => x.theme);
    let themeName: string | undefined;

    // don't suspend for the default theme
    if (globalTheme !== TextmateTheme.TOKYO_NIGHT) {
        themeName = use(getCachedThemeLoader(globalTheme));
    }

    useEffect(() => {
        if (!themeName) {
            return;
        }

        const cl = document.body.classList;

        if (cl.contains(themeName)) {
            console.warn(`theme ${TextmateTheme[globalTheme]} already in body class list`);
            return;
        }
        cl.add(themeName);
        return () => {
            cl.remove(themeName);
        };
    }, [themeName, globalTheme]);

    const contextVal = useMemo(() => ({ theme: globalTheme } satisfies ThemeContext), [globalTheme]);

    return (
        <ThemeContext value={contextVal}>
            <div className={cn(themeName, "contents")}>
                {children}
            </div>
        </ThemeContext>
    );
}

import { createServerFn } from "@tanstack/react-start";
import { getCookie, setCookie } from "@tanstack/react-start/server";

import * as z from "zod";

const COOKIE_NAME = "theme";

// import { persist } from "zustand/middleware";
// import { create } from "zustand/react";

// export interface ThemeStore {
//     theme: TextmateTheme;
// }

// export const themeStore = create<ThemeStore>()(persist(
//     (_set, _get) => {
//         return {
//             theme: TextmateTheme.TOKYO_NIGHT,
//         };
//     },
//     {
//         name: "ThemeStore",
//     },
// ));


export function getThemeForNav(): TextmateTheme {

}

const themeSchema = z.enum(TextmateTheme);

export const getThemeServerFn = createServerFn()
    .handler((): TextmateTheme => {
        const raw = getCookie(COOKIE_NAME) ?? TextmateTheme.TOKYO_NIGHT;
        const parsed = themeSchema.safeParse(raw);

        return parsed.success ? parsed.data : TextmateTheme.TOKYO_NIGHT;
    });

export const setThemeServerFn = createServerFn()
    .validator(themeSchema)
    .handler(({ data }) => {
        setCookie(COOKIE_NAME, `${data}`);
    });

