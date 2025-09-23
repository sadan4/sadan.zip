import type { ComponentProps } from "react";

export {
    Horizontal as HorizontalResizeHandle,
} from "./Horizontal";
export {
    Vertical as VerticalResizeHandle,
} from "./Vertical";

export interface ResizeHandleProps extends ComponentProps<"div"> {
}