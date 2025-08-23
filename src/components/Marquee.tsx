import { type ComponentProps, createElement } from "react";

export interface MarqueeProps extends ComponentProps<"span"> {
    behavior?: "scroll" | "slide" | "alternate";
    direction?: "left" | "right" | "up" | "down";
    loop?: number;
    scrollAmount?: number;
    scrollDelay?: number;
    trueSpeed?: boolean;
}

export function Marquee({ children, ...props }: MarqueeProps) {
    return createElement("marquee", props, children);
}
