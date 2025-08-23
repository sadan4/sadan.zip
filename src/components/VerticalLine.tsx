import cn from "@/utils/cn";

import type { ComponentProps } from "react";

export interface VerticalLineProps extends ComponentProps<"div"> {
}
/**
 * You probably want to set props in ::after
 */
export function VerticalLine({ className, ...props }: VerticalLineProps) {
    return (
        <div
            className={cn("vertical-line w-0 h-full after:w-0 after:h-full after:border-l-2 after:border-bg-fg-600/50", className)}
            {...props}
        />
    );
}
