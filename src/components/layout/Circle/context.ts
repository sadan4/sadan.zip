import { namedContext } from "@/utils/devtools";

import type { CircleItemProps } from "./Circle";


export interface CircleItemContext extends CircleItemProps {

}

export const CircleItemContext = namedContext<CircleItemContext>(null!, "CircleItemContext");

