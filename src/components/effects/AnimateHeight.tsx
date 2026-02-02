import { useForceUpdater } from "@/hooks/forceUpdater";
import { useResizeObserver } from "@/hooks/resizeObserver";
import { measureRect } from "@/utils/dom";
import { animated, useSpringValue } from "@react-spring/web";

import { type PropsWithChildren, useEffect, useRef, useState } from "react";

export interface AnimateHeightProps extends PropsWithChildren {
    animateInitialHeight?: boolean;
    show?: boolean;
}

export function AnimateHeight({ children, animateInitialHeight = false, show = true }: AnimateHeightProps) {
    const [el, setEl] = useState<HTMLDivElement | null>(null);
    const initialRender = useRef(!animateInitialHeight);
    const height = useSpringValue(animateInitialHeight ? 0 : "auto");
    const [dep, updateHeight] = useForceUpdater();

    useResizeObserver(el, updateHeight);

    useEffect(() => {
        if (el) {
            const { height: h } = measureRect(el);

            if (initialRender.current) {
                height.set(h);
            } else {
                height.start(h);
            }
            initialRender.current = false;
        }
    }, [el, height, dep]);

    return (
        <animated.div
            style={{ height }}
            className="overflow-hidden"
        >
            <div ref={setEl}>
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
