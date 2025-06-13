import type { Coord } from "@/utils/types";

import { createContext } from "react";

export const CursorPosContext = createContext({
    x: 0,
    y: 0,
} as Coord);

export const CursorClickableContext = createContext<Element | null>(null);
