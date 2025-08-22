import { cn, textColors, textSize, type TextStyleProps, textWeight } from "@/utils/cn";

import { useCursorContextStore } from "./Cursor/cursorContextStore";

import { type ComponentProps, type MouseEvent, useEffect, useRef } from "react";


interface TextAreaProps extends Omit<ComponentProps<"textarea">, "color">, TextStyleProps {
}

export function TextArea({ className, color = "white", size = "sm", weight = "normal", onMouseOver: onMouseOverProp, onMouseOut: onMouseOutProp, ...props }: TextAreaProps) {
    const shouldNullOnUnmount = useRef(false);

    useEffect(() => {
        return () => {
            if (shouldNullOnUnmount.current) {
                useCursorContextStore.getState()
                    .updateTextElement(null);
            }
        };
    }, []);

    return (
        <textarea
            className={cn("bg-bg-300 rounded-md px-3 py-1 disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 outline-none transition-[color,box-shadow] ring-1 ring-bg-fg-600/50 focus-visible:ring-3 focus:ring-bg-fg-600", textColors[color], textSize[size], textWeight[weight], className)}
            onMouseOut={(e: MouseEvent<HTMLTextAreaElement>) => {
                try {
                    onMouseOutProp?.(e as any);
                } catch (e) {
                    console.error("Error occurred in mouseOut:", e);
                }
                shouldNullOnUnmount.current = false;
                useCursorContextStore
                    .getState()
                    .updateTextElement(null);
            }}
            onMouseOver={(e: MouseEvent<HTMLTextAreaElement>) => {
                try {
                    onMouseOverProp?.(e as any);
                } catch (e) {
                    console.error("error occurred in mouseOver:", e);
                }
                shouldNullOnUnmount.current = true;
                useCursorContextStore
                    .getState()
                    .updateTextElement(e.nativeEvent.target as HTMLTextAreaElement);
            }}
            {...props}
        />
    );
}
