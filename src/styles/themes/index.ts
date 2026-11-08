import type { TextmateTheme } from "@/utils/textmate/theme";

import { getThemeDeclaration } from "./generated";

const loaderMap = new Map<TextmateTheme, Promise<string>>();

export function getCachedThemeLoader(theme: TextmateTheme): Promise<string> {
    return loaderMap.getOrInsertComputed(theme, getThemeDeclaration(theme).loader);
}
