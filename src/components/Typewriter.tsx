import { sleep } from "@/utils/async";
import { useCallback, useEffect, useImperativeHandle, useRef, useState, type ComponentPropsWithoutRef, type ReactNode, type Ref, type RefObject } from "react";
import { defaultEraser } from "./typewriterUtils";

export interface TypewriterFrame {
    component: ReactNode;
    nextDelay: number;
}

export interface TypewriterSource {
    type(): Generator<TypewriterFrame, void, ReactNode>;
    erase(prev: ReactNode): Generator<TypewriterFrame, void, ReactNode>;
}

export interface TypewriterRef {
    sendWord(source: TypewriterSource, dontDeleteOld?: boolean): void;
    isTyping: RefObject<boolean>;
}

export interface TypewriterProps extends ComponentPropsWithoutRef<"div"> {
    ref: Ref<TypewriterRef>;
    initialContent: ReactNode | TypewriterSource;
    children?: never;
    onTypingStateChange?: (newState: boolean) => void;
}

function isTypewriterSource(source: any): source is TypewriterSource {
    return Object.keys(source ?? {}).length === 2 && "type" in source && "erase" in source;
}


export default function Typewriter({ ref, initialContent, onTypingStateChange, ...props }: TypewriterProps) {
    // const [typing, setTyping] = useState(false);
    const typing = useRef(false);
    const eraser = useRef<TypewriterSource["erase"]>(defaultEraser);
    const isInitialTypewriterSource = isTypewriterSource(initialContent);
    const [content, setContent] = useState(isInitialTypewriterSource ? "" : initialContent);
    const sendWord = useCallback(async (source: TypewriterSource, dontDeleteOld?: boolean) => {
        if (typing.current) {
            console.warn("Typewriter: sendWord called while already typing");
            return;
        }
        typing.current = true;
        onTypingStateChange?.(true);
        // remove old content with the eraser
        if (content && !dontDeleteOld) {
            let con: ReactNode = content
            const gen = eraser.current(con);
            let cur = gen.next(con);
            while (!cur.done) {
                con = cur.value.component;
                setContent(con);
                await sleep(cur.value.nextDelay);
                cur = gen.next(con)
            }
        }
        
        // generate the new content

        {
            let con: ReactNode = "";
            const gen = source.type();
            let cur = gen.next(con);
            while (!cur.done) {
                con = cur.value.component;
                setContent(con);
                await sleep(cur.value.nextDelay);
                cur = gen.next(con);
            }
        }
        // set the new eraser
        eraser.current = source.erase;
        // release the lock
        typing.current = false;
        onTypingStateChange?.(false);
    }, [content, onTypingStateChange]);
    useImperativeHandle(ref, () => {
        return {
            sendWord,
            isTyping: typing
        };
    }, [sendWord]);
    useEffect(() => {
        if (isInitialTypewriterSource) {
            sendWord(initialContent);
        }
    }, [initialContent, isInitialTypewriterSource, sendWord]);
    return <div {...props} style={{
        userSelect: "none",
        ...props.style,
    }}>{content}</div>;
}