import { cn, resizeClasses, type ResizeProp, textColors, textSize, type TextStyleProps, textWeight } from "@/utils/cn";
import { updateRef } from "@/utils/ref";

import { useCursorContextStore } from "../Cursor/cursorContextStore";
import { Text } from "../Text";

import { type ComponentProps, type MouseEvent, type PropsWithChildren, useEffect, useRef } from "react";


interface TextAreaProps extends Omit<ComponentProps<"textarea">, "color">, TextStyleProps, ResizeProp {
}

export function TextArea({
    className,
    resize = "none",
    color = "white",
    size = "sm",
    weight = "normal",
    onMouseOver: onMouseOverProp,
    onMouseOut: onMouseOutProp,
    ref: _ref,
    ...props
}: TextAreaProps) {
    const shouldNullOnUnmount = useRef(false);
    const ref = useRef<HTMLTextAreaElement>(null);

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
            className={cn("bg-bg-300 rounded-md px-3 py-1 disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 outline-none transition-[color,box-shadow] ring-1 ring-fg-600/50 focus-visible:ring-3 focus:ring-fg-600", resizeClasses[resize], textColors[color], textSize[size], textWeight[weight], className)}
            onMouseOut={(e: MouseEvent<HTMLTextAreaElement>) => {
                shouldNullOnUnmount.current = false;
                useCursorContextStore
                    .getState()
                    .updateTextElement(null);
                onMouseOutProp?.(e as any);
            }}
            onMouseOver={(e: MouseEvent<HTMLTextAreaElement>) => {
                shouldNullOnUnmount.current = true;
                useCursorContextStore
                    .getState()
                    .updateTextElement(e.nativeEvent.target as HTMLTextAreaElement);
                onMouseOverProp?.(e as any);
            }}
            ref={(r) => {
                updateRef(ref, r);
                updateRef(_ref, r);
            }}
            {...props}
        />
    );
}

export interface LabeledTextAreaProps extends TextAreaProps, PropsWithChildren {
    labelColor?: keyof typeof textColors;
    labelSize?: keyof typeof textSize;
    labelWeight?: keyof typeof textWeight;
    wrapperClassName?: string;
}

export function LabeledTextArea({
    labelColor,
    labelSize = "md",
    labelWeight,
    wrapperClassName,
    children,
    ...props
}: LabeledTextAreaProps) {
    return (
        <div className={cn("flex flex-col", wrapperClassName)}>
            <Text
                color={labelColor}
                size={labelSize}
                weight={labelWeight}
            >
                {children}
            </Text>
            <TextArea {...props} />
        </div>
    );
}
