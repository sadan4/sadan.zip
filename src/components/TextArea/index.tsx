import { useComposedRefs } from "@/hooks/composedRefs";
import { cn, resizeClasses, type ResizeProp, textColors, textSize, type TextStyleProps, textWeight } from "@/utils/cn";

import { Text } from "../Text";

import { type ComponentProps, type PropsWithChildren, useRef } from "react";


interface TextAreaProps extends Omit<ComponentProps<"textarea">, "color">, TextStyleProps, ResizeProp {
}

export function TextArea({
    className,
    resize = "none",
    color = "white",
    size = "sm",
    weight = "normal",
    ref: _ref,
    ...props
}: TextAreaProps) {
    const ref = useRef<HTMLTextAreaElement>(null);
    const refs = useComposedRefs(ref, _ref);
    // FIXME: move this to scss

    return (
        <textarea
            className={cn("rounded-md bg-bg-300 px-3 py-1 ring-1 ring-fg-600/50 transition-[color,box-shadow] outline-none focus:ring-fg-600 focus-visible:ring-3 disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50", resizeClasses[resize], textColors[color], textSize[size], textWeight[weight], className)}
            ref={refs}
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
