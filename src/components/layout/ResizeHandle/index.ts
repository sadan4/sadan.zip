import type { ComponentPropsWithoutRef, Ref, RefObject } from "react";

export {
    Horizontal as HorizontalResizeHandle,
} from "./Horizontal";
export {
    Vertical as VerticalResizeHandle,
} from "./Vertical";

export interface ResizeHandleAPI {
    /**
     * @param percent - A number in the range [0, 100] indicating the position of the handle
     *
     * @param dispatchResize - Whether to dispatch the resize event, defaults to false
     */
    setCurrentPos(percent: number, dispatchResize?: boolean): void;
    reset(): void;
}

export interface ResizeHandleProps extends ComponentPropsWithoutRef<"div"> {
    /**
     * set to false to disabling resetting on double click
     */
    resetOnDoubleClick?: boolean;
    /**
     * The ref of the container for this handle
     */
    boundingElementRef: RefObject<HTMLElement | null>;
    /**
     * a number in the range [0, 1] indicating the starting position of the handle relative to the width/height of the container
     *
     * @default 0.5
     */
    initialPosition?: number;
    /**
     * a number in the range [0, 1] indicating the minimum position of the handle relative to the width/height of the container
     *
     * @default 0
     */
    minPosition?: number;
    /**
     * a number in the range [0, 1] indicating the maximum position of the handle relative to the width/height of the container
     *
     * @default 1
     */
    maxPosition?: number;
    /**
     * Called with the new percentage whenever the handle is resized
     */
    onResize?(firstWidth: number): void;
    /**
     * Called with the new percentage whenever the resizing is finished (user releases the mouse)
     */
    onResizeFinish?(firstWidth: number): void;
    /**
     * Called when the handle is reset (e.g. double-clicked)
     */
    onReset?(): void;
    ref?: Ref<ResizeHandleAPI | null>;
}
