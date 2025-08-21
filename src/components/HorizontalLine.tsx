import cn from "@/utils/cn";

import type { ComponentProps } from "react";

export interface HorizontalLineProps extends ComponentProps<"hr"> {
}

export function HorizontalLine({ className, ...props }: HorizontalLineProps) {
    return (
        <hr
            className={cn("text-bg-fg-600", className)}
            {...props}
        />
    );
}
