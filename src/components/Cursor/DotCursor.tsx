import cn from "@/utils/cn";

import styles from "./DotCursor.module.css";


export interface DotCursorProps {
    className?: string;
    radius?: number;
    invert?: boolean;
}

export function DotCursor({ className, radius = 6, invert }: DotCursorProps) {
    return (
        <div
            className={cn("rounded-full", className, invert && styles.invertCursor)}
            style={{
                width: `${radius}px`,
                height: `${radius}px`,
            }}
        />
    );
}

