import cn, { type ColorProp, textColors } from "@/utils/cn";

import type { ComponentProps } from "react";

export interface HorizontalLineProps extends Omit<ComponentProps<"hr">, "color">, ColorProp {
}

export function HorizontalLine({ className, color = "white-600", ...props }: HorizontalLineProps) {
    return (
        <hr
            className={cn("w-full my-2", textColors[color], className)}
            {...props}
        />
    );
}
