import type { CircleItemProps } from "./Circle";

import { createContext } from "react";


export interface CircleItemContext extends CircleItemProps {

}

export const CircleItemContext = createContext<CircleItemContext>(null!);
CircleItemContext.displayName = "CircleItemContext";

