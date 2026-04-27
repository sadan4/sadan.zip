import { imageTypewriter, makeTextComponentEraser, textComponentTypewriter, Typewriter, type TypewriterFrame, type TypewriterImage, type TypewriterRef, type TypewriterSource } from "@/components/effects/Typewriter";
import { REPLACEMENT_CHARACTER } from "@/utils/constants";
import { randInt } from "@/utils/math";

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
    "salad",
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
    textComponentTypewriter(75, REPLACEMENT_CHARACTER.repeat(9), nameTextProps),
];

const NAME = "sadan";
const INITIAL_NAME = <Text {...nameTextProps}>{NAME}</Text>;

function clickMe(): TypewriterSource {
    return {
        *type() {
            yield {
                component: INITIAL_NAME,
                nextDelay: 1000,
            };

            yield* makeTextComponentEraser(NAME, 50, nameTextProps)();

            const clickMeFrames = textComponentTypewriter(50, "Click Me!", nameTextProps);

            let _val: TypewriterFrame = {
                component: "",
                nextDelay: 0,
            };

            for (const val of clickMeFrames.type()) {
                yield _val = val;
            }
            yield {
                ..._val,
                nextDelay: 750,
            };

            yield* clickMeFrames.erase(_val.component);

            yield* textComponentTypewriter(50, NAME, nameTextProps).type();
        },
        erase: makeTextComponentEraser(NAME, 50, nameTextProps),
    };
}

const possibleNames = possibleNameStrings
    .map((str) => (typeof str === "string" ? textComponentTypewriter(50, str, nameTextProps) : str))
    .concat(possibleImages.map((img) => imageTypewriter(img, nameTextProps)));

export default function Name() {
    const typewriterRef = useRef<TypewriterRef>(null);
    const lastIndexRef = useRef(-1);
    const [typing, setTyping] = useState(false);

    // TODO: this is a bit cursed
    useEffect(() => {
        let timeout: NodeJS.Timeout;

        function tryStart() {
            if (typewriterRef.current) {
                typewriterRef.current.sendWord(clickMe(), true);
            } else {
                timeout = setTimeout(tryStart, 10);
            }
        }

        tryStart();


        return () => clearTimeout(timeout);
    }, []);
    return (
        <div
            className="max-w-3xl min-w-24 text-center"
            style={{
                cursor: typing ? "not-allowed" : "pointer",
            }}
        >
            <Typewriter
                className="mt-6 mb-6 flex min-h-10 justify-center text-balance break-all"
                initialContent={INITIAL_NAME}
                onTypingStateChange={(prevState) => {
                    setTyping(prevState);
                }}
                ref={typewriterRef}
                onClick={() => {
                    if (typing)
                        return;

                    let idx: number;

                    // TODO: just a tad cursed
                    while ((idx = randInt(0, possibleNames.length)) === lastIndexRef.current)
                        ;

                    typewriterRef.current?.sendWord(possibleNames[idx]);
                }}
            />
        </div>
    );
}
