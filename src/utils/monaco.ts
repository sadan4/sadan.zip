import { extensionForLanguage } from "./textmate/language";
import { error } from "./error";
import { getLanguageDeps, Language } from "./textmate";

import * as monaco from "monaco-editor";

/**
 * null == undefined
 */
export function cmpUri(uri1: monaco.Uri | null | undefined, uri2: monaco.Uri | null | undefined): boolean {
    if (uri1 == null) {
        return uri2 == null;
    }
    if (uri2 == null) {
        return false;
    }
    return uri1 === uri2 || uri1.toString() === uri2.toString();
}

export function cmpModel(
    model1: monaco.editor.ITextModel | null | undefined,
    model2: monaco.editor.ITextModel | null | undefined,
): boolean {
    if (model1 == null) {
        return model2 == null;
    }
    if (model2 == null) {
        return false;
    }
    return model1 === model2 || model1.id === model2.id;
}

export function updateModelLanguage(model: monaco.editor.ITextModel, language: Language) {
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

export function isReadOnly(editor: monaco.editor.ICodeEditor): boolean {
    return editor.getOption(monaco.editor.EditorOption.readOnly);
}

let id = 0;

export function uriForLanguage(language: Language): monaco.Uri {
    return monaco.Uri.file(`source-${++id}${extensionForLanguage(language)}`);
}
