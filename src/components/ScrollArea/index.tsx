import cn from "@/utils/cn";
import type { ComponentPropsWithRef } from "react";

export enum ScrollAreaType {
    AUTO,
    ALWAYS,
    SCROLL,
    HIDDEN,
}

export interface ScrollAreaProps extends ComponentPropsWithRef<"div"> {
    type?: ScrollAreaType;
    hideDelay?: number;
}

export function ScrollArea({ type, hideDelay = 600, children, className, ...props }: ScrollAreaProps) {
    return (
        <div
            className={cn("min-w-full table", className)}
            {...props}
        >
            {children}
        </div>
    );
}

