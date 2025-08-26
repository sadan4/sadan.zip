import { createContext, type PropsWithChildren, type ReactNode } from "react";

interface SelectContext {

}

const SelectContext = createContext<SelectContext | null>(null);

export interface SelectProps extends PropsWithChildren {
}

export function Select({}: SelectProps) {
    return null;
}

export function SelectContent() {
    return null;
}
