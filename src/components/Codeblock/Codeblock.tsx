import { ScrollArea } from "@/components/layout/ScrollArea";
import { useDeepState } from "@/hooks/deepState";
import { copy } from "@/utils/clipboard";
import cn from "@/utils/cn";
import { assert } from "@/utils/error";
import * as shiki from "@/utils/shiki";
import { Language } from "@/utils/textmate";
import { languageDisplayNames } from "@/utils/textmate/language";
import { TextmateTheme } from "@/utils/textmate/theme";
import type { LineNumberColor } from "@/utils/textmate/themes";

import { highlightCode } from "./_internal/highlightCode";
import { HorizontalOverflowMode } from "./enums";
import styles from "./styles.module.scss";
import { IconButton } from "../Button";
import { ScrollAreaDirection } from "../layout/ScrollArea/types";
import { Text } from "../Text";

import { CopyIcon } from "lucide-react";
import { type ComponentProps, startTransition, Suspense, useCallback, useEffect, useState } from "react";

export interface CodeblockProps extends Omit<ComponentProps<"div">, "children" | "lang"> {
    children: string;
    lang: Language;
    theme?: TextmateTheme;
    lineNumbers?: boolean;
    /**
     * implies `lineNumbers ??= true`
     *
     * default 1 if {@link lineNumbers} is true
     *
     * @see {@link lineNumbers}
     */
    startingLineNumber?: number;
    /**
     * {@link HorizontalOverflowMode.WRAP} requires {@link lineNumbers} to be true
     */
    overflowX?: HorizontalOverflowMode;
    noCopy?: boolean;
}

declare module "react" {
    interface CSSProperties {
        "--line-num-fg"?: string;
        "--line-num-active-fg"?: string;
    }
}

function CodeblockInner({
    lang,
    theme = TextmateTheme.TOKYO_NIGHT,
    children,
    className,
    overflowX = HorizontalOverflowMode.WRAP,
    startingLineNumber,
    lineNumbers: _lineNumbers,
    noCopy = false,
    ...props
}: CodeblockProps) {
    // NOTE: react compiler workaround
    let lineNumbers = _lineNumbers;

    if (lineNumbers === undefined) {
        lineNumbers = startingLineNumber != null ? true : undefined;
    }
    if (overflowX === HorizontalOverflowMode.WRAP) {
        assert(lineNumbers !== false, "lineNumbers must not be false when overflowX is WRAP");
        lineNumbers = true;
    }

    const highlightToHtml = useCallback(() => {
        assert(shiki.highlighter, "Highlighter not loaded");
        return highlightCode({
            lang,
            theme,
            highlighter: shiki.highlighter,
            code: children,
            showLineNumbers: lineNumbers,
            startingLineNumber: startingLineNumber ?? 1,
        });
    }, [children, lang, lineNumbers, startingLineNumber, theme]);

    // in ssr, the themes/grammars are always loaded
    const [html, setHtml] = useState<string>(() => (import.meta.env.SSR ? highlightToHtml() : ""));

    const [{ foreground, activeForeground }, setLineNumberColor]
    = useDeepState<LineNumberColor>(import.meta.env.SSR ? shiki.getLineNumberColor(theme) : {});

    useEffect(() => {
        // don't suspend if we dont need to
        if (shiki.themeNeedsLoad(theme) || shiki.grammarNeedsLoad(lang)) {
            startTransition(async () => {
                await Promise.all([shiki.loadTheme(theme), shiki.loadGrammar(lang)]);

                startTransition(() => {
                    setHtml(highlightToHtml());
                    setLineNumberColor(shiki.getLineNumberColor(theme));
                });
            });
        } else {
            setHtml(highlightToHtml());
            setLineNumberColor(shiki.getLineNumberColor(theme));
        }
    }, [setLineNumberColor, highlightToHtml, lang, theme]);

    let highlightedCode = (
        <div
            // eslint-disable-next-line @eslint-react/dom/no-dangerously-set-innerhtml -- from shiki
            dangerouslySetInnerHTML={{ __html: html }}
            // we want a mismatch to avoid showing the fallback on suspense
            suppressHydrationWarning
            className={styles.code}
            style={{
                "--line-num-fg": foreground,
                "--line-num-active-fg": activeForeground,
            }}
        />
    );

    switch (overflowX) {
        case HorizontalOverflowMode.SCROLL: {
            highlightedCode = (
                <ScrollArea
                    className="max-h-[unset]"
                    dir={ScrollAreaDirection.HORIZONTAL}
                >
                    {highlightedCode}
                </ScrollArea>
            );
            break;
        }
        case HorizontalOverflowMode.WRAP: {
            className = cn(className, styles.wrap);
            break;
        }
        case HorizontalOverflowMode.CLIP: break;
    }

    return (
        <div
            {...props}
            className={cn(styles.codeblockWrapper, className)}
        >
            {highlightedCode}
            <div className={styles.overlayContainer}>
                <Text>{languageDisplayNames[lang]}</Text>
                {!noCopy && (
                    <div className={styles.buttonContainer}>
                        <IconButton
                            label="Copy"
                            // FIXME: notice / something with error
                            onClick={() => copy(children)
                                .then(() => true)
                                .catch(() => false)}
                        >
                            <CopyIcon />
                        </IconButton>
                    </div>
                )}
            </div>
        </div>
    );
}

export function Codeblock(props: CodeblockProps) {
    return (
        <Suspense name="Codeblock">
            <CodeblockInner {...props} />
        </Suspense>
    );
}
