import { animated, useSpring } from "@react-spring/web";
import { useHover } from "@use-gesture/react";

import type { PropsWithChildren } from "react";

export interface HoverScaleProps extends PropsWithChildren {
    factor?: number;
}

export default function HoverScale({ factor = 1.1, children }: HoverScaleProps) {
    const [{ scale }, api] = useSpring(() => ({
        scale: 1,
    }));

    const bind = useHover(({ hovering }) => {
        api.start({ scale: hovering ? factor : 1 });
    });

    return (
        <animated.div
            style={{
                scale,
            }}
            {...bind()}
        >
            {children}
        </animated.div>
    );
}
