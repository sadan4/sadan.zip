import { range } from "@/utils/functional";

import { Text, type TextProps, type TextTags } from "./Text";
import Typewriter, { type TypewriterFrame, type TypewriterRef, type TypewriterSource } from "./Typewriter";
import { defaultEraser } from "./typewriterUtils";

import { type ReactNode, useCallback, useEffect, useRef, useState } from "react";

const nameTextProps: TextProps<"span"> = {
    color: "accent",
    size: "4xl",
    weight: "bold",
};

function stringTypewriter(nextDelay: number, string: string): TypewriterSource {
    return {
        *type() {
            for (let i = 1; i <= string.length; i++) {
                yield {
                    component: `${string.substring(0, i)}|`,
                    nextDelay,
                };
            }
            yield {
                component: string,
                nextDelay,
            };
        },
        erase: defaultEraser,
    };
}

function textComponentTypewriter(
    nextDelay: number,
    str: string,
    extraProps: Omit<TextProps<"span">, "children"> = {},
): TypewriterSource {
    return {
        *type() {
            for (let i = 1; i <= str.length; i++) {
                yield {
                    component: <Text {...extraProps}>{str.substring(0, i)}|</Text>,
                    nextDelay,
                };
            }
            yield {
                component: <Text {...extraProps}>{str}</Text>,
                nextDelay,
            };
        },
        erase: makeTextComponentEraser(str, nextDelay, extraProps),
    };
}

function makeTextComponentEraser(
    prevStr: string,
    nextDelay: number,
    extraProps: TextProps<TextTags>,
) {
    return function *(): Generator<TypewriterFrame, void, ReactNode> {
        let cur = prevStr;

        while (cur.length > 0) {
            yield {
                component: <Text {...extraProps}>{cur = cur.substring(0, cur.length - 1)}|</Text>,
                nextDelay,
            };
        }
        yield {
            component: (
                <Text
                    {...extraProps}
                    children=""
                />
            ),
            nextDelay,
        };
    };
}

interface TypewriterImage {
    htmlTag: string;
    alt: string;
    href: string;
}

function makeDiscordEmojiImage(emojiId: string, emojiName: string): TypewriterImage {
    return {
        htmlTag: `<img src="https://cdn.discordapp.com/emojis/${emojiId}.webp?size=128" alt=":${emojiName}:"></img>`,
        alt: `:${emojiName}:`,
        href: `https://cdn.discordapp.com/emojis/${emojiId}.webp?size=128`,
    };
}

const possibleImages: TypewriterImage[] = [
    makeDiscordEmojiImage("1026533070955872337", "blobcatcozy"),
    {
        htmlTag: `<img src="/assets/creature.png" alt="creature"></img>`,
        alt: "creature",
        href: "/assets/creature.png",
    },
    makeDiscordEmojiImage("1026532993923293184", "husk"),
    makeDiscordEmojiImage("1320236763494486087", "steamcatcozy"),
    makeDiscordEmojiImage("1262562427422244874", "wires"),
];

function imageTypewriter(image: TypewriterImage, extraProps: TextProps<TextTags>): TypewriterSource {
    return {
        *type() {
            for (let i = 1; i <= image.htmlTag.length; i++) {
                yield {
                    component: <Text {...extraProps}>{image.htmlTag.substring(0, i)}|</Text>,
                    nextDelay: 35,
                };
            }
            yield {
                component: (
                    <img
                        className="w-32 h-32"
                        src={image.href}
                        alt={image.alt}
                    />
                ),
                nextDelay: 0,
            };
        },
        erase: makeTextComponentEraser(image.htmlTag, 35, extraProps),
    };
}

const possibleNameStrings = [
    "sadan",
    ":3",
    "hiiiii",
    "minecraft addict",
    "save the world player",
    "linux user",
    "WOMP WOMP",
    "avid ozone fan",
    "i use NixOS, btw",
    "Lazily Evaluated",
    "Reproducible",
    "Declarative",
    "Open Source",
    ":husk:",
    ":blobcatcozy:",
    ":wires:",
    "Hop on Vencord",
];

function clickMe(orig: string): TypewriterSource {
    return {
        *type() {
            yield {
                component: <Text {...nameTextProps}>{orig}</Text>,
                nextDelay: 1000,
            };
            for (const val of makeTextComponentEraser(orig, 50, nameTextProps)()) {
                yield val;
            }

            const clickMeFrames = textComponentTypewriter(50, "Click Me!", nameTextProps);

            let _val: TypewriterFrame = {
                component: "",
                nextDelay: 0,
            };

            for (const val of clickMeFrames.type()) {
                _val = val;
                yield val;
            }
            yield {
                ..._val,
                nextDelay: 750,
            };
            for (const val of clickMeFrames.erase(_val.component)) {
                yield val;
            }

            const origFrames = textComponentTypewriter(50, orig, nameTextProps);

            for (const val of origFrames.type()) {
                yield val;
            }
        },
        erase: makeTextComponentEraser(orig, 50, nameTextProps),
    };
}

const possibleNames = possibleNameStrings
    .map((str) => textComponentTypewriter(50, str, nameTextProps))
    .concat(possibleImages.map((img) => imageTypewriter(img, nameTextProps)));

export default function Name() {
    const typewriterRef = useRef<TypewriterRef>(null);
    const lastIndex = useRef(-1);
    const [typing, setTyping] = useState(false);

    const onTypingStateChange = useCallback((typingState: boolean) => {
        setTyping(typingState);
    }, []);

    useEffect(() => {
        const tryStart = () => {
            if (typewriterRef.current) {
                typewriterRef.current.sendWord(clickMe("sadan"), true);
            } else {
                setTimeout(tryStart, 10);
            }
        };

        return clearTimeout.bind(null, setTimeout(tryStart, 10));
    }, []);
    return (
        <div
            className="text-center min-w-24 max-w-3xl"
            style={{
                cursor: typing ? "not-allowed" : "pointer",
            }}
        >
            <Typewriter
                className="text-4xl font-bold mt-6 mb-6 min-h-10 text-accent break-words text-balance"
                initialContent="sadan"
                onTypingStateChange={onTypingStateChange}
                ref={typewriterRef}
                onClick={() => {
                    if (typing)
                        return;

                    let idx: number;

                    while ((idx = range(0, possibleNames.length)) === lastIndex.current) {
                        // guh
                    }
                    typewriterRef.current?.sendWord(possibleNames[idx]);
                }}
            />
        </div>
    );
}
