import { getThemeDeclaration } from "@/styles/themes/generated";
import { cn } from "@/utils/cn";
import { useRouteContext, useRouter } from "@tanstack/react-router";

import { setCachedTheme, setThemeServerFn, ThemeContext } from "./misc";

import { type PropsWithChildren, startTransition, useEffect, useMemo, useOptimistic, useRef } from "react";
import { preinit } from "react-dom";


export function RootThemeProvider({ children }: PropsWithChildren) {
    const rootCx = useRouteContext({ from: "__root__" });
    const [theme, setOptimisticTheme] = useOptimistic(rootCx.theme);
    const reqRef = useRef(0);
    const router = useRouter();
    const decl = getThemeDeclaration(theme);

    useEffect(() => {
        setCachedTheme(rootCx.theme);
    }, [rootCx.theme]);

    const contextVal = useMemo(() => ({
        theme,
        updateTheme(newTheme) {
            const decl = getThemeDeclaration(newTheme);

            if (!import.meta.env.SSR) {
                preinit(decl.href, {
                    as: "style",
                    fetchPriority: "high",
                });
            }
            setCachedTheme(newTheme);

            const id = ++reqRef.current;

            setCachedTheme(newTheme);
            startTransition(async () => {
                setOptimisticTheme(newTheme);
                // we have to invalidate optimistically so it doesn't flash
                await router.invalidate();
                await setThemeServerFn({ data: newTheme });
                if (id === reqRef.current) {
                    await router.invalidate();
                }
            });
        },
    } satisfies ThemeContext), [theme]);

    return (
        <ThemeContext value={contextVal}>
            <link
                rel="stylesheet"
                href={decl.href}
            />
            <div className={cn(decl.className, "contents")}>
                {children}
            </div>
        </ThemeContext>
    );
}
