import cn from "@/utils/cn";
import { updateRef } from "@/utils/ref";

import { ScrollAreaContext } from "./context";
import styles from "./styles.module.scss";
import { ScrollAreaDirection, type ScrollAreaType } from "./types";

import { type ComponentPropsWithRef, useRef } from "react";

export interface ScrollAreaProps extends Omit<ComponentPropsWithRef<"div">, "dir"> {
    type?: ScrollAreaType;
    hideDelay?: number;
    dir?: ScrollAreaDirection;
}

const directionStyles: Record<ScrollAreaDirection, string> = {
    [ScrollAreaDirection.BOTH]: styles.both,
    [ScrollAreaDirection.HORIZONTAL]: styles.horizontal,
    [ScrollAreaDirection.VERTICAL]: styles.vertical,
};


export function ScrollArea({
    dir = ScrollAreaDirection.VERTICAL,
    children,
    className,
    ref: _ref,
    ...props
}: ScrollAreaProps) {
    const ref = useRef<HTMLDivElement | null>(null);

    return (
        <ScrollAreaContext.Provider value={{ ref }}>
            <div
                className={cn(styles.scrollbar, directionStyles[dir], className)}
                ref={(e) => {
                    updateRef(ref, e);
                    updateRef(_ref, e);
                }}
                {...props}
            >
                {children}
            </div>
        </ScrollAreaContext.Provider>
    );
}
