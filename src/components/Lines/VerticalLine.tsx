import cn from "@/utils/cn";

import * as styles from "./styles.module.scss";

import type { ComponentProps } from "react";

export interface VerticalLineProps extends ComponentProps<"div"> {
}
/**
 * You probably want to set props in ::after
 */
export function VerticalLine({ className, ...props }: VerticalLineProps) {
    return (
        <div
            className={cn(styles.vr, className)}
            {...props}
        />
    );
}
