import type { Coord } from "@/utils/types";

import { createContext } from "react";

export const CursorClickableContext = createContext<Element | null>(null);

export const lastPos: Coord = {
    x: 0,
    y: 0,
};

document.addEventListener("mousemove", ({ clientX, clientY }) => {
    lastPos.x = clientX;
    lastPos.y = clientY;
});
