import { imageTypewriter, makeTextComponentEraser, textComponentTypewriter, Typewriter, type TypewriterFrame, type TypewriterImage, type TypewriterRef, type TypewriterSource } from "@/components/effects/Typewriter";
import { range } from "@/utils/functional";

import { Text, type TextProps } from "./Text";

import { useEffect, useRef, useState } from "react";

const nameTextProps: TextProps<"span"> = {
    color: "accent",
    size: "4xl",
    weight: "bold",
};


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
            className="max-w-3xl min-w-24 text-center"
            style={{
                cursor: typing ? "not-allowed" : "pointer",
            }}
        >
            <Typewriter
                className="mt-6 mb-6 flex min-h-10 justify-center text-balance break-words"
                initialContent="sadan"
                onTypingStateChange={(prevState) => {
                    setTyping(prevState);
                }}
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
