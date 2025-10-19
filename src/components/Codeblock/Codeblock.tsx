import cn from "@/utils/cn";
import { assert, unreachable } from "@/utils/error";

import styles from "./styles.module.scss";
import { CodeblockLang, HorizontalOverflowMode } from ".";
import { Button } from "../Button";
import { ScrollArea } from "../layout/ScrollArea";
import { ScrollAreaDirection } from "../layout/ScrollArea/types";
import { Tooltip } from "../Tooltip";

import { CopyIcon } from "lucide-react";
import { type ComponentProps, useState } from "react";
import ShikiHighlighter, { createHighlighterCore, createOnigurumaEngine, type Language } from "react-shiki/core";
import type { HighlighterCore } from "shiki";
import { copy } from "@/utils/clipboard";

export interface CodeblockProps extends Omit<ComponentProps<"div">, "children" | "lang"> {
    children: string;
    lang: CodeblockLang;
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

// FIXME: setup proper HMR
function useHighlighter(): [undefined, false] | [HighlighterCore, true] {
    globalThis[Symbol.for("__highlighterPromise")] ??= createHighlighterCore({
        themes: [import("@shikijs/themes/tokyo-night")],
        langs: [import("@shikijs/langs/tsx"), import("@shikijs/langs/html")],
        engine: createOnigurumaEngine(import("shiki/wasm")),
    });

    const [hl, setHl] = useState<HighlighterCore>(globalThis[Symbol.for("__highlighter")]);

    if (!hl) {
        globalThis[Symbol.for("__highlighterPromise")].then((highlighter) => setHl(globalThis[Symbol.for("__highlighter")] = highlighter));
    }

    // @ts-expect-error
    return [hl, hl !== undefined];
}

const langMap: Record<CodeblockLang, Language> = {
    [CodeblockLang.HTML]: "html",
    [CodeblockLang.TSX]: "tsx",
};

export function Codeblock({
    lang,
    children,
    className,
    overflowX = HorizontalOverflowMode.WRAP,
    startingLineNumber,
    lineNumbers = startingLineNumber != null ? true : undefined,
    noCopy = false,
    ...props
}: CodeblockProps) {
    if (overflowX === HorizontalOverflowMode.WRAP) {
        assert(lineNumbers !== false, "lineNumbers must not be false when overflowX is WRAP");
        lineNumbers = true;
    }

    const highlighter = useHighlighter();

    if (!highlighter[1]) {
        return;
    }

    let hl = (
        <ShikiHighlighter
            highlighter={highlighter[0]}
            theme="tokyo-night"
            language={langMap[lang]}
            showLineNumbers={lineNumbers}
            startingLineNumber={startingLineNumber ?? 1}
        >
            {children}
        </ShikiHighlighter>
    );

    switch (overflowX) {
        case HorizontalOverflowMode.SCROLL: {
            hl = (
                <ScrollArea
                    className="max-h-[unset]"
                    dir={ScrollAreaDirection.HORIZONTAL}
                >
                    {hl}
                </ScrollArea>
            );
            break;
        }
        case HorizontalOverflowMode.WRAP: {
            className = cn(className, styles.wrap);
            break;
        }
        case HorizontalOverflowMode.CLIP: break;
        default: {
            unreachable();
        }
    }

    return (
        <div
            {...props}
            className={cn(styles.codeblock, className)}
        >
            {hl}
            {
                !noCopy && (
                    <div className={styles.overlayContainer}>
                        <div className={styles.buttonContainer}>
                            <Tooltip text="Copy">
                                <Button onClick={() => copy(children)}>
                                    <CopyIcon />
                                </Button>
                            </Tooltip>
                        </div>
                    </div>
                )
            }
        </div>
    );
}
