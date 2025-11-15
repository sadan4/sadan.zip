import { copy } from "@/utils/clipboard";
import cn from "@/utils/cn";
import { assert, unreachable } from "@/utils/error";
import { Language } from "@/utils/textmate";
import { ScrollArea } from "@components/layout/ScrollArea";

import { useHighlighter } from "./_internal/useHighlighter";
import { HorizontalOverflowMode } from "./enums";
import styles from "./styles.module.scss";
import { Button } from "../Button";
import { ScrollAreaDirection } from "../layout/ScrollArea/types";
import { Tooltip } from "../Tooltip";

import { CopyIcon } from "lucide-react";
import { type ComponentProps, Suspense } from "react";
import * as shiki from "react-shiki/core";

export interface CodeblockProps extends Omit<ComponentProps<"div">, "children" | "lang"> {
    children: string;
    lang: Language;
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

const langMap: Record<Language, shiki.Language> = {
    [Language.HTML]: "html",
    [Language.JSON]: "json",
    [Language.JAVASCRIPT]: "javascript",
    [Language.JAVASCRIPT_REACT]: "jsx",
    [Language.TYPESCRIPT]: "typescript",
    [Language.TYPESCRIPT_REACT]: "tsx",
    [Language.PLAINTEXT]: "plaintext",
    [Language.UNKNOWN]: "plaintext",
};

function CodeblockInner({
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

    const highlighter = useHighlighter(lang);

    let hl = (
        <shiki.ShikiHighlighter
            highlighter={highlighter}
            theme="tokyo-night"
            language={langMap[lang]}
            showLineNumbers={lineNumbers}
            startingLineNumber={startingLineNumber ?? 1}
        >
            {children}
        </shiki.ShikiHighlighter>
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

export function Codeblock(props: CodeblockProps) {
    return (
        <Suspense fallback={null}>
            <CodeblockInner {...props} />
        </Suspense>
    );
}
