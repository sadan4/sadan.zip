import cn from "@/utils/cn";
import type { ComponentPropsWithRef } from "@react-spring/web";

import type { ReactNode } from "react";

export interface BoxProps extends ComponentPropsWithRef<"div"> {
    children: ReactNode;
}

export function Box({ children, className, ...props }: BoxProps) {
    return (
        <div
            className={cn("flex flex-col items-center gap-2 bg-bg-100 p-4 rounded-md outline-2 outline-bg-fg-600/25", className)}
            {...props}
        >
            {children}
        </div>
    );
}
