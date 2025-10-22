import { measureRect } from "@/utils/dom";
import toCSS from "@/utils/toCSS";
import { animated, to, useSpring } from "@react-spring/web";
import { useGesture } from "@use-gesture/react";

import { type PropsWithChildren, useRef } from "react";

export interface PerspectiveHoverProps extends PropsWithChildren {
    /**
     * lower number -> bigger effect
     */
    hoverFactor: number;
}

export default function PerspectiveHover({ children, hoverFactor }: PerspectiveHoverProps) {
    function calcX(pointerY: number, height: number, posY: number): number {
        return -(pointerY - posY - (height / 2)) / hoverFactor;
    }
    function calcY(pointerX: number, width: number, posX: number): number {
        return (pointerX - posX - (width / 2)) / hoverFactor;
    }

    const domRef = useRef<HTMLDivElement>(null);

    const [{ x, y, scale, zoom, rotateX, rotateY, rotateZ }, api] = useSpring(() => ({
        rotateX: 0,
        rotateY: 0,
        rotateZ: 0,
        scale: 1,
        zoom: 0,
        x: 0,
        y: 0,
        config: {
            mass: 5,
            friction: 40,
            tension: 350,
        },
    }));

    useGesture({
        onMove({ xy: [pointerX, pointerY], dragging, down }) {
            if (dragging || !domRef.current)
                return;

            const { width, height, x, y } = measureRect(domRef.current);

            api.start({
                rotateX: calcX(pointerY, height, y),
                rotateY: calcY(pointerX, width, x),
                scale: down ? 1 : 1.1,
            });
        },
        onHover({ hovering }) {
            if (hovering)
                return;
            api.start({
                rotateX: 0,
                rotateY: 0,
                scale: 1,
            });
        },
        onMouseDown() {
            api.start({
                scale: 1,
            });
        },
        onMouseUp() {
            api.start({
                scale: 1.1,
            });
        },
        onMouseOut() {
            api.start({
                scale: 1,
            });
        },
    }, {
        target: domRef,
        eventOptions: {
            passive: false,
        },
    });

    return (
        <animated.div
            ref={domRef}
            style={{
                transform: toCSS.perspective(600),
                x,
                y,
                scale: to([scale, zoom], (s, z) => s + z),
                rotateX,
                rotateY,
                rotateZ,
            }}
            className="touch-none"
        >
            {children}
        </animated.div>
    );
}
