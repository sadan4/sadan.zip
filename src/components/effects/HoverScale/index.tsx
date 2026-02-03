import { animated, useSpringValue } from "@react-spring/web";

import type { PropsWithChildren } from "react";

export interface HoverScaleProps extends PropsWithChildren {
    factor?: number;
}

export default function HoverScale({ factor = 1.1, children }: HoverScaleProps) {
    const scale = useSpringValue(1);

    return (
        <animated.div
            style={{
                scale,
            }}
            onMouseOver={() => {
                scale.start({ to: factor });
            }}
            onMouseOut={() => {
                scale.start({ to: 1 });
            }}
        >
            {children}
        </animated.div>
    );
}
