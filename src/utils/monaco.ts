import { extensionForLanguage } from "./textmate/language";
import { error, unavailableImport } from "./error";
import { getLanguageDeps, Language } from "./textmate";

import type * as Monaco from "monaco-editor";
export {
    Monaco,
};
export const monaco: typeof import("monaco-editor") = import.meta.env.SSR ? unavailableImport("monaco-editor") : await import("monaco-editor");

/**
 * null == undefined
 */
export function cmpUri(uri1: Monaco.Uri | null | undefined, uri2: Monaco.Uri | null | undefined): boolean {
    if (uri1 == null) {
        return uri2 == null;
    }
    if (uri2 == null) {
        return false;
    }
    return uri1 === uri2 || uri1.toString() === uri2.toString();
}

export function cmpModel(
    model1: Monaco.editor.ITextModel | null | undefined,
    model2: Monaco.editor.ITextModel | null | undefined,
): boolean {
    if (model1 == null) {
        return model2 == null;
    }
    if (model2 == null) {
        return false;
    }
    return model1 === model2 || model1.id === model2.id;
}

export function updateModelLanguage(model: Monaco.editor.ITextModel, language: Language) {
    monaco.editor.setModelLanguage(model, getMonacoLanguageString(language));
}

const monacoLanguageStringMap: Readonly<Record<Language, string>> = Object.freeze({
    [Language.TYPESCRIPT]: "typescript",
    [Language.TYPESCRIPT_REACT]: "typescript",
    [Language.JAVASCRIPT]: "javascript",
    [Language.JAVASCRIPT_REACT]: "javascript",
    [Language.JSON]: "json",
    [Language.HTML]: "html",
    [Language.PLAINTEXT]: "plaintext",
    [Language.UNKNOWN]: "plaintext",
    [Language.CSS]: "css",
} satisfies Record<Language, string>);

export function getMonacoLanguageString(language: Language): string {
    return monacoLanguageStringMap[language] || error(`unsupported language: ${language}`);
}

export function registerMonacoLanguage(language: Language) {
    monaco.languages.register({ id: getMonacoLanguageString(language) });
}

export function makeTMLanguageMap(language: Language): Map<string, string> {
    const deps = getLanguageDeps(language)
        .map((lang) => [getMonacoLanguageString(lang), lang] as const);

    return new Map(deps);
}

export function isReadOnly(editor: Monaco.editor.ICodeEditor): boolean {
    return editor.getOption(monaco.editor.EditorOption.readOnly);
}

let id = 0;

export function uriForLanguage(language: Language): Monaco.Uri {
    return monaco.Uri.file(`source-${++id}${extensionForLanguage(language)}`);
}
