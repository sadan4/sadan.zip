import type { Ref } from "react";

export const enum Language {
    PLAINTEXT = "plaintext",
    UNKNOWN = "",
    JSON = "json",
    TYPESCRIPT = "typescript",
    JAVASCRIPT = "javascript",
    TYPESCRIPT_REACT = "typescriptreact",
    JAVASCRIPT_REACT = "javascriptreact",
}

export interface CodeEditorProps<THandle> {
    initialCode?: string;
    onChange?(newCode: string): void;
    language?: Language;
    width?: string;
    height?: string;
    className?: string;
    ref?: Ref<THandle>;
}

