import { Clickable } from "@/components/Clickable";
import { sleep } from "@/utils/async";

import { defaultEraser, type TypewriterRef, type TypewriterSource } from ".";

import { type ComponentPropsWithoutRef, type ReactNode, type Ref, type RefObject, type SetStateAction, type Dispatch, useCallback, useEffect, useId, useImperativeHandle, useRef, useState } from "react";

type Guh = ComponentPropsWithoutRef<"div">;

export interface TypewriterProps extends Guh {
    ref: Ref<TypewriterRef>;
    initialContent: ReactNode | TypewriterSource;
    children?: never;
    onTypingStateChange?: (newState: boolean) => void;
}

function isTypewriterSource(source: any): source is TypewriterSource {
    return Object.keys(source ?? {}).length === 2 && "type" in source && "erase" in source;
}

type SendWord = (source: TypewriterSource, dontDeleteOld?: boolean | undefined) => void;

function makeSendWord(
    content: ReactNode,
    onTypingStateChange?: TypewriterProps["onTypingStateChange"],
    typingRef: RefObject<boolean>,
    eraserRef: RefObject<TypewriterSource["erase"]>,
    setContent: Dispatch<SetStateAction<ReactNode>>,
): SendWord {
    return async (source: TypewriterSource, dontDeleteOld?: boolean | undefined) => {
        if (typingRef.current) {
            console.warn("Typewriter: sendWord called while already typing");
            return;
        }
        typingRef.current = true;
        onTypingStateChange?.(true);
        // remove old content with the eraser
        if (content && !dontDeleteOld) {
            let con: ReactNode = content;
            const gen = eraserRef.current(con);
            let cur = gen.next(con);

            while (!cur.done) {
                con = cur.value.component;
                setContent(con);
                await sleep(cur.value.nextDelay);
                cur = gen.next(con);
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
        eraserRef.current = source.erase;
        // release the lock
        typingRef.current = false;
        onTypingStateChange?.(false);
    }
}


export component Typewriter({ ref, initialContent, onTypingStateChange, ...props }: TypewriterProps) {
    const typingRef = useRef(false);
    const eraserRef = useRef<TypewriterSource["erase"]>(defaultEraser);
    const isInitialTypewriterSource = isTypewriterSource(initialContent);
    const [content, setContent] = useState(isInitialTypewriterSource ? "" : initialContent);
    const id = useId();

    const sendWord = useCallback<SendWord>(makeSendWord(content, onTypingStateChange, typingRef, eraserRef, setContent), [content, onTypingStateChange]);

    useImperativeHandle(ref, () => {
        return {
            sendWord,
            isTyping: typingRef,
        };
    }, [sendWord]);
    useEffect(() => {
        if (isInitialTypewriterSource) {
            sendWord(initialContent, undefined);
        }
    }, [initialContent, isInitialTypewriterSource, sendWord]);
    <Clickable
        id={id}
        tag="div"
        {...props}
        style={{
            userSelect: "none",
            ...props.style,
        }}
        aria-labelledby={id}
    >
        {content}
    </Clickable>
}
