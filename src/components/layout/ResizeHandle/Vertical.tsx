import cn from "@/utils/cn";

import type { ResizeHandleProps } from ".";
import styles from "./styles.module.scss";

export interface VerticalResizeHandleProps extends ResizeHandleProps {

}

export function Vertical({ className, ...props }: VerticalResizeHandleProps) {
    return (
        <div
            className={cn(styles.verticalHandle, className)}
            {...props}
        />
    );
}
