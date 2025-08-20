import cn from "@/utils/cn";

import { useCursorContextStore } from "./Cursor/cursorContextStore";

import { type ChangeEventHandler, type ComponentProps, useEffect, useRef } from "react";

const inputSizes = {
    sm: "text-sm",
    md: "text-md",
    lg: "text-lg",
    xl: "text-xl",
    "2xl": "text-2xl",
    "3xl": "text-3xl",
} as const;

export interface InputProps extends ComponentProps<"input"> {
    textSize?: keyof typeof inputSizes;
    value: string;
    onChange: ChangeEventHandler<HTMLInputElement>;
}

export function Input({ className, textSize, onMouseOver, onMouseOut, ...props }: InputProps) {
    const shouldNullOnDemount = useRef(false);

    useEffect(() => {
        if (shouldNullOnDemount.current) {
            useCursorContextStore
                .getState()
                .updateTextElement(null);
        }
    }, []);
    return (
        <input
            type="text"
            name="demangled text input"
            className={cn("bg-bg-300 rounded-md px-3 py-1 w-full disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 outline-none transition-[color,box-shadow] focus-visible:ring-3 ring-bg-fg-600", inputSizes[textSize ?? "md"], className)}
            onMouseOver={(e) => {
                try {
                    onMouseOver?.(e);
                } catch (e) {
                    console.error("Error occurred in onMouseOver:", e);
                }
                shouldNullOnDemount.current = true;
                useCursorContextStore
                    .getState()
                    .updateTextElement(e.nativeEvent.target as Element);
            }}
            onMouseOut={(e) => {
                try {
                    onMouseOut?.(e);
                } catch (e) {
                    console.error("Error occurred in onMouseOut:", e);
                }
                shouldNullOnDemount.current = false;
                useCursorContextStore
                    .getState()
                    .updateTextElement(null);
            }}

            {...props}
        />
    );
}
