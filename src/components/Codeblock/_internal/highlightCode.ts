import { assert } from "@/utils/error";
import { langMap, themeMap } from "@/utils/shiki";
import type { Language } from "@/utils/textmate";
import type { TextmateTheme } from "@/utils/textmate/theme";
import { truthy } from "@/utils/types";

import * as styles from "../styles.module.scss";

import type { HighlighterCore, ShikiTransformer } from "shiki/core";

export interface HighlighterOptions {
    highlighter: HighlighterCore;
    /**
     * Whether to show line numbers
     * @default false
     */
    showLineNumbers?: boolean;
    /**
     * Starting line number (when showLineNumbers is true)
     * @default 1
     */
    startingLineNumber?: number;
    code: string;
    lang: Language;
    theme: TextmateTheme;
}

function makeLineNumberTransformer(startingLineNumber = 1): ShikiTransformer {
    let maxDigits = 1;

    return {
        name: "sadan:line-numbers",
        preprocess(code) {
            maxDigits = String(code.split(/\r?\n/).length + startingLineNumber - 1).length;

            return code;
        },
        code(node) {
            // TODO: probably not needed, remove
            this.addClassToHast(node, styles.lineNumbersContainer);

            let style = node.properties.style ?? "";

            assert(!Array.isArray(style));

            style = `--num-max-digits: ${maxDigits}; ${style}`;

            if (startingLineNumber !== 1) {
                style = `--line-start: ${startingLineNumber}; ${style}`;
            }

            node.properties = {
                ...node.properties,
                style,
            };
        },
        line(node) {
            this.addClassToHast(node, styles.line);
            return node;
        },
    };
}

/**
 * Base hook for syntax highlighting using Shiki.
 * This is the core implementation used by all entry points.
 *
 * @param code - The code to highlight
 * @param lang - Language for highlighting
 * @param theme - Theme or themes to use
 * @param options - Highlighting options
 * @param highlighterFactory - Factory function to create highlighter (internal use)
 */
export function highlightCode({
    code,
    lang,
    theme,
    highlighter,
    showLineNumbers,
    startingLineNumber,
}: HighlighterOptions) {
    const transformers = [showLineNumbers && makeLineNumberTransformer(startingLineNumber)].filter(truthy);

    const html = highlighter.codeToHtml(code, {
        theme: themeMap[theme],
        lang: langMap[lang],
        transformers,
    });

    return html;
}
