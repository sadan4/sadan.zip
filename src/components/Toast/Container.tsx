import { ToastContext, useNewToastStore } from "@/stores/ToastStore";

import { Toaster } from "./Toaster";

import { type PropsWithChildren, useRef } from "react";

export interface ToastContainerProps extends PropsWithChildren {

}

export function ToastContainer({ children }: ToastContainerProps) {
    const ref = useRef<HTMLDivElement>(null);
    const store = useNewToastStore();

    return (
        <ToastContext value={store}>
            <div ref={ref}>
                {children}
                <Toaster containerRef={ref} />
            </div>
        </ToastContext>
    );
}
