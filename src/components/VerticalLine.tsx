import cn from "@/utils/cn";

import type { ComponentProps } from "react";

export interface VerticalLineProps extends ComponentProps<"div"> {
}
export function VerticalLine({ className, ...props }: VerticalLineProps) {
    return (
        <div
            className={cn("vertical-line", className)}
            {...props}
        />
    );
}
