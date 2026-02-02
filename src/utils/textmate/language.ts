import { error } from "../error";

export enum Language {
    PLAINTEXT = "source.txt",
    UNKNOWN = "",
    JSON = "source.json",
    TYPESCRIPT = "source.ts",
    JAVASCRIPT = "source.js",
    TYPESCRIPT_REACT = "source.tsx",
    JAVASCRIPT_REACT = "source.jsx",
    HTML = "source.html",
    CSS = "source.css",
}

const extensionMap: Readonly<Record<Language, string>> = Object.freeze({
    [Language.PLAINTEXT]: "txt",
    [Language.UNKNOWN]: "",
    [Language.JSON]: "json",
    [Language.TYPESCRIPT]: "ts",
    [Language.JAVASCRIPT]: "js",
    [Language.TYPESCRIPT_REACT]: "tsx",
    [Language.JAVASCRIPT_REACT]: "jsx",
    [Language.HTML]: "html",
    [Language.CSS]: "css",
});

export function extensionForLanguage(language: Language): string {
    const ext = extensionMap[language];

    return (ext && `.${ext}`) ?? error(`Could not find extension for ${language}`);
}

export const languageDisplayNames = Object.freeze({
    [Language.PLAINTEXT]: "txt",
    [Language.UNKNOWN]: "UNKNOWN",
    [Language.JSON]: "json",
    [Language.TYPESCRIPT]: "ts",
    [Language.JAVASCRIPT]: "js",
    [Language.TYPESCRIPT_REACT]: "tsx",
    [Language.JAVASCRIPT_REACT]: "jsx",
    [Language.HTML]: "html",
    [Language.CSS]: "css",
} satisfies Record<Language, string>);
