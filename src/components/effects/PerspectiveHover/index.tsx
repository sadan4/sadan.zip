import { MouseButtons } from "@/utils/dom";
import { measureRect } from "@/utils/dom/rect";
import { animated, to, useSpring } from "@react-spring/web";

import { type PropsWithChildren, useRef } from "react";

export interface PerspectiveHoverProps extends PropsWithChildren {
    /**
     * lower number -> bigger effect
     */
    hoverFactor: number;
    className?: string;
}

export default function PerspectiveHover({ children, hoverFactor, className }: PerspectiveHoverProps) {
    function calcX(pointerY: number, height: number, posY: number): number {
        return -(pointerY - posY - (height / 2)) / hoverFactor;
    }
    function calcY(pointerX: number, width: number, posX: number): number {
        return (pointerX - posX - (width / 2)) / hoverFactor;
    }

    const ref = useRef<HTMLDivElement>(null);
    const rectRef = useRef<Record<"width" | "height" | "x" | "y", number>>(null);

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

    return (
        <animated.div
            ref={ref}
            style={{
                transform: "perspective(600px)",
                x,
                y,
                scale: to([scale, zoom], (s, z) => s + z),
                rotateX,
                rotateY,
                rotateZ,
            }}
            className={className}
            onMouseMove={(e) => {
                if (!ref.current) {
                    return;
                }

                const { width, height, x, y } = rectRef.current ?? measureRect(ref.current);

                api.start({
                    rotateX: calcX(e.clientY, height, y),
                    rotateY: calcY(e.clientX, width, x),
                    scale: e.buttons & MouseButtons.PRIMARY ? 1 : 1.1,
                });
            }}
            onMouseDown={() => {
                api.start({
                    scale: 1,
                });
            }}
            onMouseUp={() => {
                api.start({
                    scale: 1.1,
                });
            }}
            onMouseOver={() => {
                if (ref.current) {
                    rectRef.current = measureRect(ref.current);
                }
            }}
            onMouseOut={() => {
                api.start({
                    rotateX: 0,
                    rotateY: 0,
                    scale: 1,
                });
            }}
        >
            {children}
        </animated.div>
    );
}
