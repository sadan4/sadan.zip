import { cn, colors, textSize, type TextStyleProps, textWeight } from "@/utils/cn";

import type { ComponentProps } from "react";


interface TextAreaProps extends Omit<ComponentProps<"textarea">, "color">, TextStyleProps {
}

export function TextArea({ className, color = "white", size = "sm", weight = "normal", ...props }: TextAreaProps) {
    return (
        <textarea
            className={cn("bg-bg-300 rounded-md px-3 py-1 disabled:pointer-events-none disabled:cursor-not-allowed disabled:opacity-50 outline-none transition-[color,box-shadow] focus-visible:ring-3 ring-bg-fg-600", colors[color], textSize[size], textWeight[weight], className)}
            {...props}
        />
    );
}
