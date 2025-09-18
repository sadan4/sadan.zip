import cn from "@/utils/cn";
import { animated, type ComponentPropsWithRef } from "@react-spring/web";

import styles from "./styles.module.css";

import type { ReactNode } from "react";

export interface BoxProps extends ComponentPropsWithRef<"div"> {
    children: ReactNode;
}

export const AnimatedBox = animated(Box);

export function Box({ children, className, ...props }: BoxProps) {
    return (
        <div
            className={cn(styles.box, className)}
            {...props}
        >
            {children}
        </div>
    );
}
