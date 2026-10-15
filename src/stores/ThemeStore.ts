import { TextmateTheme } from "@/utils/textmate/theme";
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
