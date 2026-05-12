import { Boilerplate } from "@/components/Boilerplate";
import { Button } from "@/components/Button";
import { Box } from "@/components/layout/Box";
import { Text } from "@/components/Text";
import { TextArea } from "@/components/TextArea";
import { ToggleButtonGroup } from "@/components/ToggleButtonGroup";
import { Tooltip } from "@/components/Tooltip";
import { useForceUpdater } from "@/hooks/forceUpdater";
import { useResizeObserverFromRef } from "@/hooks/resizeObserver";
import { fill } from "@/utils/array";
import { paste } from "@/utils/clipboard";
import cn from "@/utils/cn";
import { NBSP } from "@/utils/constants";
import { assert } from "@/utils/error";
import { createFileRoute } from "@tanstack/react-router";

import defaultJson from "./defaultJson.txt?raw";
import defaultSource from "./defaultSource.txt?raw";
import * as styles from "./styles.module.scss";

import { AlertCircleIcon, BracesIcon, FileIcon } from "lucide-react";
import { Fragment, type ReactNode, useEffect, useLayoutEffect, useRef, useState } from "react";

interface Token {
    kind: string;
    span: {
        start: number;
        end: number;
    };
    flags: string;
    contents(): ReactNode;
}

interface RawInput {
    tokens: RawToken[];
}


interface RawToken extends Omit<Token, "contents"> {
}

export const Route = createFileRoute("/_/vis/")({
    component: Vis,
});

const knownColors: Record<string, string> = {
    Comment: cn("border-info-600"),
    NewLine: cn("border-0 border-transparent"),
    StringContents: cn("border-success-800 bg-success-300/50"),
    Whitespace: cn("border-info-400/50 bg-transparent"),
    Eq: cn("border-accent-800 bg-accent-300/50"),
    Semi: cn("border-accent-800 bg-accent-300/50"),
    RedirectFdOutput: cn("border-accent-800 bg-accent-300/50"),
    Dollar: cn("border-accent-800 bg-accent-300/50"),
    DollarBrace: cn("border-accent-800 bg-accent-300/50"),
    LBrace: cn("border-accent-800 bg-accent-300/50"),
    RBrace: cn("border-accent-800 bg-accent-300/50"),
    LBracket: cn("border-accent-800 bg-accent-300/50"),
    RBracket: cn("border-accent-800 bg-accent-300/50"),
    LParen: cn("border-accent-800 bg-accent-300/50"),
    RParen: cn("border-accent-800 bg-accent-300/50"),
    RAngle: cn("border-accent-800 bg-accent-300/50"),
    LAngle: cn("border-accent-800 bg-accent-300/50"),
    OrOr: cn("border-accent-800 bg-accent-300/50"),
    Pipe: cn("border-accent-800 bg-accent-300/50"),
    And: cn("border-accent-800 bg-accent-300/50"),
    Grave: cn("border-accent-800 bg-accent-300/50"),
    Quote: cn("border-accent-800 bg-accent-300/50"),
    DoubleQuote: cn("border-accent-800 bg-accent-300/50"),
    Command: cn("border-secondary-900 bg-secondary-400/50"),
    Ident: cn("border-warning-900 bg-warning-400/50"),
    // TODO: color once lexer supports these
    Minus: cn("border-info-600 bg-info-600/50"),
    Decrement: cn("border-info-600 bg-info-600/50"),
    Colon: cn("border-info-600 bg-info-600/50"),
    Slash: cn("border-info-600 bg-info-600/50"),
    Unknown: cn("border-error-900 bg-error-400/50"),
};

function colorForType(type: string) {
    if (!(type in knownColors)) {
        console.error("missing color", type);

        return cn("border-info-600 bg-info-600/50");
    }

    return knownColors[type];
}

function EmptyToken(count: number) {
    const svgRef = useRef<SVGSVGElement>(null);
    const rectRef = useRef<SVGRectElement>(null);
    const [dep, updateAngle] = useForceUpdater();

    useResizeObserverFromRef(svgRef, updateAngle);
    useLayoutEffect(() => {
        if (svgRef.current && rectRef.current) {
            const { width, height } = svgRef.current.getBoundingClientRect();

            svgRef.current.style.setProperty("--width", `${width}px`);
            svgRef.current.style.setProperty("--height", `${height}px`);
        }
    }, [dep]);

    return (
        <>
            {fill(count + 1, NBSP).join("")}
            <svg
                ref={svgRef}
                className="absolute inset-fill fill-error-400"
            >
                <rect
                    ref={rectRef}
                    className={styles.emptyToken}
                />
            </svg>
        </>
    );
}

function parseTokens(json: string, source: string): Token[] {
    const arr: RawInput = JSON.parse(json);

    assert(Array.isArray(arr.tokens), "Expected an array");

    return arr.tokens.map(({ kind, span, flags }) => {
        const tokenColor = colorForType(kind);
        const contents = source.substring(span.start, span.end);

        return {
            kind,
            span,
            flags,
            contents(): ReactNode {
                return (
                    <Tooltip
                        text={(
                            <ul className="text-left">
                                <li>
                                    {kind}
                                </li>
                                <li>
                                    [{span.start}, {span.end})
                                </li>
                                {
                                    !contents && (
                                        <li>
                                            <span className="">
                                                <AlertCircleIcon className="inline h-4 w-auto stroke-warning-300 pr-1 align-text-top" />This token has no content!
                                            </span>
                                        </li>
                                    )
                                }
                            </ul>
                        )}
                        className="inline"
                    >
                        <span
                            className={cn("relative border align-middle", tokenColor)}
                        >
                            {kind === "NewLine" ? <br /> : contents || (!!(span.end - span.start) && EmptyToken(span.end - span.start))}
                        </span>
                    </Tooltip>
                );
            },
        } satisfies Token;
    });
}

function Vis() {
    const [text, setText] = useState("");
    const [source, setSource] = useState("");
    const [tokens, setTokens] = useState<Token[]>([]);
    const [tab, setTab] = useState<"json" | "code">("json");
    // react refresh hack

    function updateTokens() {
        if (text) {
            try {
                setTokens(parseTokens(text, source));
            } catch {
                // noop
            }
        } else {
            setTokens([]);
        }
    }

    useEffect(updateTokens, [source, text]);

    return (
        <>
            <Boilerplate />
            <div className="flex h-full w-full flex-col items-center pt-[20vh]">
                <Text
                    size="4xl"
                    color="accent"
                >
                    Token Visualizer
                </Text>
                <div className="mt-6 flex w-9/10 flex-col items-center gap-6">
                    <ToggleButtonGroup
                        className="m-2 rounded-lg border-2 border-fg-700 bg-bg-200 p-2"
                        onSelectItem={(item) => setTab(item)}
                        items={[
                            {
                                id: "json",
                                label: "JSON",
                                renderIcon() {
                                    return <BracesIcon />;
                                },
                            },
                            {
                                id: "code",
                                label: "Code",
                                renderIcon() {
                                    return <FileIcon />;
                                },
                            },
                        ]}
                    />
                    <TextArea
                        size="lg"
                        value={tab === "json" ? text : source}
                        onChange={(e) => {
                            if (tab === "json") {
                                setText(e.target.value);
                            } else {
                                setSource(e.target.value);
                            }
                        }}
                        placeholder='some json here'
                        className="h-[10vh] max-h-[25vh] min-h-20 w-[60vw] max-w-[60vw] min-w-50 resize"
                    />
                    <div className="flex h-9 gap-3">
                        <Button
                            onClick={() => {
                                paste().then(setText);
                            }}
                        >
                            Paste
                        </Button>
                        <Button
                            color="secondary"
                            colorType="outline"
                            onClick={() => {
                                setText(defaultJson);
                                setSource(defaultSource);
                            }}
                        >
                            Fill With Example
                        </Button>
                    </div>
                    <Box className="inline w-full [&>*:not(:first-child)]:ml-0.5">
                        {
                            tokens.length ? tokens.map((t) => <Fragment key={`${t.kind}-${t.span.start}`}><t.contents /></Fragment>) : null
                        }
                    </Box>
                </div>
            </div>
        </>
    );
}

