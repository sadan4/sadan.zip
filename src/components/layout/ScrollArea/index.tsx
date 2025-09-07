import { base } from "@/styles";
import cn from "@/utils/cn";
import { updateRef } from "@/utils/ref";

import { ScrollAreaContext } from "./context";
import styles from "./styles.module.css";
import type { ScrollAreaType } from "./types";

import { type ComponentPropsWithRef, useRef } from "react";

export interface ScrollAreaProps extends ComponentPropsWithRef<"div"> {
    type?: ScrollAreaType;
    hideDelay?: number;
}


export function ScrollArea({ children, className, ref: _ref, ...props }: ScrollAreaProps) {
    const ref = useRef<HTMLDivElement | null>(null);

    return (
        <ScrollAreaContext.Provider value={{ ref }}>
            <div
                className={cn("min-w-full relative", base.maxH400Px, styles.scrollbar, className)}
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

