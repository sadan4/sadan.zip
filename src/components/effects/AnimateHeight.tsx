import { useForceUpdater } from "@/hooks/forceUpdater";
import useResizeObserver from "@react-hook/resize-observer";
import { animated, useSpringValue } from "@react-spring/web";

import { type PropsWithChildren, useEffect, useRef } from "react";

export interface AnimateHeightProps extends PropsWithChildren {
    animateInitialHeight?: boolean;
    show?: boolean;
}

export function AnimateHeight({ children, animateInitialHeight = false, show = true }: AnimateHeightProps) {
    const ref = useRef<HTMLDivElement>(null);
    const initialRender = useRef(!animateInitialHeight);
    const height = useSpringValue(0);
    const [dep, updateHeight] = useForceUpdater();

    useResizeObserver(ref, updateHeight);

    useEffect(() => {
        if (ref.current) {
            const { height: h } = ref.current.getBoundingClientRect();

            if (initialRender.current) {
                height.set(h);
            } else {
                height.start(h);
            }
            initialRender.current = false;
        }
    }, [height, dep]);

    return (
        <animated.div
            style={{ height }}
            className="overflow-hidden"
        >
            <div ref={ref}>
                <div
                    style={{
                        height: show ? "auto" : 0,
                    }}
                >
                    {children}
                </div>
            </div>
        </animated.div>
    );
}
