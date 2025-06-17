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
    function calcX(pointerY: number, posY: number): number {
        return -(pointerY - posY - (window.innerHeight / 2)) / hoverFactor;
    }
    function calcY(pointerX: number, posX: number): number {
        return (pointerX - posX - (window.innerWidth / 2)) / hoverFactor;
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
        onDrag({ active, offset: [x, y] }) {
            api({
                x,
                y,
                rotateX: 0,
                rotateY: 0,
                scale: active ? 1 : 1.1,
            });
        },
        onMove({ xy: [pointerX, pointerY], dragging }) {
            if (dragging)
                return;
            api({
                rotateX: calcX(pointerY, y.get()),
                rotateY: calcY(pointerX, x.get()),
                scale: 1.1,
            });
        },
        onHover({ hovering }) {
            if (hovering)
                return;
            api({
                rotateX: 0,
                rotateY: 0,
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
        >
            {children}
        </animated.div>
    );
}
