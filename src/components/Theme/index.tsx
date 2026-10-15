import { getCachedThemeLoader } from "@/styles/themes";
import { cn } from "@/utils/cn";
import { TextmateTheme } from "@/utils/textmate/theme";

import { ThemeContext } from "./context";

import { type PropsWithChildren, startTransition, use, useEffect, useMemo, useOptimistic, useRef } from "react";

export function RootThemeProvider({ children }: PropsWithChildren) {
    const rootCx = useRouteContext({ from: "__root__" });
    const [theme, setOptimisticTheme]  = useOptimistic(rootCx.theme);
    let themeName: string | undefined;
    const reqRef = useRef(0);
    const router = useRouter();

    const contextVal = useMemo(() => ({ theme, updateTheme(newTheme) {
        setThemeForNav(newTheme);
        let id = ++reqRef.current;
        setThemeForNav(newTheme);
        startTransition(async () => {
            setOptimisticTheme(newTheme);
            await setThemeServerFn({data: newTheme });
            if (id === reqRef.current) {
                await router.invalidate();
            }
        })
    }, } satisfies ThemeContext), [theme]);

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
import { useRouteContext, useRouter } from "@tanstack/react-router";

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

let themeCache = TextmateTheme.TOKYO_NIGHT;

export function getThemeForNav(): TextmateTheme {
    return themeCache;
}

export function setThemeForNav(t: TextmateTheme) {
    themeCache = t;
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

