import cn from "@/utils/cn";

import type { ChangeEventHandler, ComponentProps } from "react";

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

export function Input({ className, textSize, ...props }: InputProps) {
    return (
        <input
            type="text"
            name="demangled text input"
            className={cn("bg-bg-300 rounded-md", inputSizes[textSize ?? "md"], className)}
            {...props}
        />
    );
}
