import { base } from "@/styles";
import cn, { bgColors } from "@/utils/cn";

import type { ComponentProps } from "react";

export interface HorizontalLineProps extends Omit<ComponentProps<"hr">, "color"> {
    color?: keyof typeof bgColors;
}

export function HorizontalLine({ className, color = "white-600", ...props }: HorizontalLineProps) {
    return (
        <div
            className={cn(base.wFull, base.my2, base.h1Px, bgColors[color], className)}
            {...props}
        />
    );
}
