import { Boilerplate } from "@/components/Boilerplate";
import { Button } from "@/components/Button";
import { DefaultFooter, FooterContainer } from "@/components/Footer";
import { Box } from "@/components/layout/Box";
import { Text } from "@/components/Text";
import { TextArea } from "@/components/TextArea";
import { Tooltip } from "@/components/Tooltip";
import { paste } from "@/utils/clipboard";

import defaultJson from "./default.json?raw";

import invariant from "invariant";
import { Fragment, type ReactNode, useEffect, useState } from "react";

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

interface TokenColor {
    bg: string;
    border: string;
}

const colors = new Map<string, TokenColor>();

function colorForType(type: string) {
    if (!colors.has(type)) {
        const randomHex = Math.floor(Math.random() * (0xffffff + 1))
            .toString(16)
            .padStart(6, "0");

        const bg = `color-mix(in oklab, #${randomHex} 50%, transparent)`;
        const border = `color-mix(in oklab, #${randomHex} 80%, transparent)`;

        colors.set(type, {
            bg,
            border,
        });
    }

    return colors.get(type)!;
}

const REMOVE_FQN_REGEX = /.*\./;

function parseTokens(json: string): Token[] {
    const arr: RawToken[] = JSON.parse(json);

    invariant(Array.isArray(arr), "Expected an array");

    return arr.map(({ contents, pos, type }) => {
        const parsedType = type.replace(REMOVE_FQN_REGEX, "");
        const tokenColor = colorForType(parsedType);

        return {
            type: parsedType,
            pos,
            contents(): ReactNode {
                return (
                    <Tooltip
                        text={parsedType}
                        className="inline"
                        triggerClassName="inline"
                    >
                        <span
                            style={{
                                backgroundColor: tokenColor.bg,
                                borderColor: tokenColor.border,
                            }}
                            className="border font-mono"
                        >
                            {contents}
                        </span>
                    </Tooltip>
                );
            },
        };
    });
}

export default function Vis() {
    const [text, setText] = useState("");
    const [tokens, setTokens] = useState<Token[]>([]);

    function updateTokens() {
        if (text) {
            try {
                setTokens(parseTokens(text));
            } catch {
                // noop
            }
        } else {
            setTokens([]);
        }
    }
    useEffect(updateTokens, [text]);

    return (
        <>
            <Boilerplate />
            <FooterContainer
                footer={() => <DefaultFooter />}
            >
                <div className="flex h-full w-full flex-col items-center pt-[20vh]">
                    <Text
                        size="4xl"
                        color="accent"
                    >
                        Token Visualizer
                    </Text>
                    <div className="mt-6 flex w-1/3 flex-col items-center gap-6">
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
                            <Button
                                color="secondary"
                                colorType="text"
                                onClick={() => {
                                    colors.clear();
                                    updateTokens();
                                }}
                            >
                                Reroll Colors
                            </Button>
                        </div>
                        <Box className="inline-block w-full">
                            {
                                tokens.length ? tokens.map((t) => <Fragment key={`${t.type}-${t.pos.start}`}><t.contents /></Fragment>) : null
                            }
                        </Box>
                    </div>
                </div>
            </FooterContainer>
        </>
    );
}
