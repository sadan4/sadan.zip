import cn, { bgColors } from "@/utils/cn";

import styles from "./styles.module.scss";

import type { ComponentProps } from "react";

export interface HorizontalLineProps extends Omit<ComponentProps<"hr">, "color"> {
    color?: keyof typeof bgColors;
}

export function HorizontalLine({ className, color = "white-600", ...props }: HorizontalLineProps) {
    return (
        <div
            className={cn(styles.hr, bgColors[color], className)}
            {...props}
        />
    );
}
