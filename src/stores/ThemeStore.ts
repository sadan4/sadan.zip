import { TextmateTheme } from "@/utils/textmate/theme";

import { persist } from "zustand/middleware";
import { create } from "zustand/react";

export interface ThemeStore {
    theme: TextmateTheme;
}

export const themeStore = create<ThemeStore>()(persist(
    (_set, _get) => {
        return {
            theme: TextmateTheme.TOKYO_NIGHT,
        };
    },
    {
        name: "ThemeStore",
    },
));
