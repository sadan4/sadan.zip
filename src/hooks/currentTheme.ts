import { ThemeContext } from "@/components/Theme/misc";
import type { TextmateTheme } from "@/utils/textmate/theme";

import { use } from "react";

export function useCurrentTheme(): TextmateTheme {
    return use(ThemeContext).theme;
}
