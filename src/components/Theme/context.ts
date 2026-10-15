import type { TextmateTheme } from "@/utils/textmate/theme";

import { createContext } from "react";

export const ThemeContext = createContext<ThemeContext>(null!);

ThemeContext.displayName = "ThemeContext";

export interface ThemeContext {
    theme: TextmateTheme;
}
