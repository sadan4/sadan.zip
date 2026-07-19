import { createContext } from "react";

export interface ModalContext {
    open(): void;
    close(): void;
    status: boolean;
    requestClose(): void;
}

export const ModalContext = createContext<ModalContext | null>(null);
ModalContext.displayName = "ModalContext";
