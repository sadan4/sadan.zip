import { useForceUpdater } from "@/hooks/forceUpdater";
import { useOnce } from "@/hooks/once";
import { useSize } from "@/hooks/size";
import toCSS from "@/utils/toCSS";
import { animated, to, useSpring } from "@react-spring/web";
import { useGesture } from "@use-gesture/react";

import { type PropsWithChildren, useEffect, useRef } from "react";

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
    const [sizeDep, _forceSizeUpdate] = useForceUpdater();


    const [{ x, y, scale, zoom, rotateX, rotateY, rotateZ, width, height, elX, elY }, api] = useSpring(() => ({
        rotateX: 0,
        rotateY: 0,
        rotateZ: 0,
        scale: 1,
        zoom: 0,
        x: 0,
        y: 0,
        elX: 0,
        elY: 0,
        width: 0,
        height: 0,
        config: {
            mass: 5,
            friction: 40,
            tension: 350,
        },
    }));

    const ensureDimsSet = useOnce(() => {
        if (!domRef.current) {
            console.warn("animating before the element is mounted");
            return;
        }

        const box = domRef.current.getBoundingClientRect();

        elX.set(box.x);
        elY.set(box.y);
        width.set(box.width);
        height.set(box.height);
    });

    {
        const { x: elX, y: elY, width, height } = useSize(() => domRef.current) ?? {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };

        useEffect(() => {
            api.start({
                elX,
                elY,
                width,
                height,
            });
        }, [api, elX, elY, height, width, sizeDep]);
    }

    useGesture({
        onMove({ xy: [pointerX, pointerY], dragging, down }) {
            if (dragging)
                return;
            if (width.get() === 0 || height.get() === 0) {
                ensureDimsSet();
            }
            api.start({
                rotateX: calcX(pointerY, width.get(), elY.get()),
                rotateY: calcY(pointerX, height.get(), elX.get()),
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
