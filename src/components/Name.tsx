import { useCallback, useEffect, useRef, useState } from "react";
import Typewriter, { type TypewriterFrame, type TypewriterRef, type TypewriterSource } from "./Typewriter";
import { defaultEraser } from "./typewriterUtils";
import { range, sample } from "lodash";
import invariant from "invariant";

function stringTypewriter(nextDelay: number, string: string): TypewriterSource {

  return {
    *type() {
      for (let i = 1; i <= string.length; i++) {
        yield {
          component: `${string.substring(0, i)}|`,
          nextDelay
        };
      }
      yield {
        component: string,
        nextDelay
      };
    },
    erase: defaultEraser
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
    href: `https://cdn.discordapp.com/emojis/${emojiId}.webp?size=128`
  };
}

const possibleImages: TypewriterImage[] = [
  makeDiscordEmojiImage("1026533070955872337", "blobcatcozy"),
  {
    htmlTag: `<img src="/assets/creature.png" alt="creature"></img>`,
    alt: "creature",
    href: "/assets/creature.png"
  },
  makeDiscordEmojiImage("1026532993923293184", "husk"),
  makeDiscordEmojiImage("1320236763494486087", "steamcatcozy"),
  makeDiscordEmojiImage("1262562427422244874", "wires")
];

function imageTypewriter(image: TypewriterImage): TypewriterSource {
  return {
    *type() {
      for (let i = 1; i <= image.htmlTag.length; i++) {
        yield {
          component: `${image.htmlTag.substring(0, i)}|`,
          nextDelay: 35
        };
      }
      yield {
        component: <img className="max-w-32" src={image.href} alt={image.alt} />,
        nextDelay: 0
      };
    },
    erase: defaultEraser.bind(null, image.htmlTag, 35),
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
        component: orig,
        nextDelay: 1000
      };
      for (const val of defaultEraser(orig)) {
        yield val;
      }
      const clickmeTyper = stringTypewriter(50, "Click Me!");
      let _val: TypewriterFrame = {
        component: "",
        nextDelay: 0
      };
      for (const val of clickmeTyper.type()) {
        _val = val;
        yield val;
      }
      yield {
        ..._val,
        nextDelay: 750
      };
      for (const val of clickmeTyper.erase(_val.component)) {
        _val = val;
        yield val;
      }
      const origTyper = stringTypewriter(50, orig);
      for (const val of origTyper.type()) {
        yield val;
      }
    },
    erase: defaultEraser
  };
}

const possibleNames = possibleNameStrings.map(stringTypewriter.bind(null, 50))
  .concat(possibleImages.map(imageTypewriter));
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
        typewriterRef.current.sendWord(clickMe("sadan"), true)
      } else {
        setTimeout(tryStart, 10);
      }
    }
    setTimeout(tryStart, 10);
  }, []);
  return <div className="text-center min-w-24 max-w-3xl"
    style={{
      cursor: typing ? "not-allowed" : "pointer"
    }} >
    <Typewriter className="text-4xl font-bold mt-6 mb-6 min-h-10 text-accent break-words text-balance" initialContent="sadan"
      onTypingStateChange={onTypingStateChange}
      ref={typewriterRef} onClick={() => {
        const possibleIndexes = range(0, possibleNames.length);
        let idx = sample(possibleIndexes);
        while (idx == lastIndex.current) {
          idx = sample(possibleIndexes);
        }
        invariant(idx && idx >= 0, "this should never happen");
        typewriterRef.current?.sendWord(possibleNames[idx]);
      }} />
  </div>;
}
