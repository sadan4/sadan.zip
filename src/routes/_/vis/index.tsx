import { Boilerplate } from "@/components/Boilerplate";
import { Button } from "@/components/Button";
import { Box } from "@/components/layout/Box";
import { Text } from "@/components/Text";
import { TextArea } from "@/components/TextArea";
import { Tooltip } from "@/components/Tooltip";
import { useForceUpdater } from "@/hooks/forceUpdater";
import { useReiszeObserverFromRef } from "@/hooks/resizeObserver";
import { fill } from "@/utils/array";
import { paste } from "@/utils/clipboard";
import cn from "@/utils/cn";
import { NBSP } from "@/utils/constants";
import { assert } from "@/utils/error";
import { createFileRoute } from "@tanstack/react-router";

import defaultJson from "./default.json?raw";
import styles from "./styles.module.scss";

import { AlertCircleIcon } from "lucide-react";
import { Fragment, type ReactNode, useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";

interface Token {
    type: string;
    pos: {
        start: number;
        length: number;
    };
    contents(): ReactNode;
}

interface RawToken extends Omit<Token, "contents"> {
    contents: string;
}

export const Route = createFileRoute("/_/vis/")({
    component: Vis,
});

const knownColors: Record<string, string> = {
    LiteralToken: cn("border-accent-300 bg-accent-300/50"),
    BlankSpaceToken: cn("border-info-500 bg-transparent"),
    MinusToken: cn("border-warning-300 bg-warning-300/60"),
    EofToken: cn("border-error-400 bg-transparent"),
    DoubleQuoteToken: cn("border-info-300 bg-info-300/50"),
};

function colorForType(type: string) {
    if (!(type in knownColors)) {
        console.error("missing color", type);

        return cn("border-info-600 bg-info-600/50");
    }

    return knownColors[type];
}

const REMOVE_FQN_REGEX = /.*\./;

function EmptyToken(count: number) {
    const svgRef = useRef<SVGSVGElement>(null);
    const rectRef = useRef<SVGRectElement>(null);
    const [dep, updateAngle] = useForceUpdater();

    useReiszeObserverFromRef(svgRef, updateAngle);
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

function parseTokens(json: string): Token[] {
    const arr: RawToken[] = JSON.parse(json);

    assert(Array.isArray(arr), "Expected an array");

    return arr.map(({ contents, pos, type }) => {
        const parsedType = type.replace(REMOVE_FQN_REGEX, "");
        const tokenColor = colorForType(parsedType);

        return {
            type: parsedType,
            pos,
            contents(): ReactNode {
                return (
                    <Tooltip
                        text={(
                            <ul className="text-left">
                                <li>
                                    {parsedType}
                                </li>
                                <li>
                                    [{pos.start}, {pos.start + pos.length})
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
                            {contents || EmptyToken(pos.length)}
                        </span>
                    </Tooltip>
                );
            },
        } satisfies Token;
    });
}

function Vis() {
    const [text, setText] = useState("");
    const [tokens, setTokens] = useState<Token[]>([]);
    // react refresh hack

    const updateTokens = useCallback(() => {
        if (text) {
            try {
                setTokens(parseTokens(text));
            } catch {
                // noop
            }
        } else {
            setTokens([]);
        }
    }, [text]);

    useEffect(updateTokens, [updateTokens]);

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
                    <TextArea
                        size="lg"
                        value={text}
                        onChange={(e) => {
                            setText(e.target.value);
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
                            }}
                        >
                            Fill With Example
                        </Button>
                    </div>
                    <Box className="inline w-full [&>*:not(:first-child)]:ml-0.5">
                        {
                            tokens.length ? tokens.map((t) => <Fragment key={`${t.type}-${t.pos.start}`}><t.contents /></Fragment>) : null
                        }
                    </Box>
                </div>
            </div>
        </>
    );
}

