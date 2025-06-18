import { useSize } from "@/hooks/size";
import cn from "@/utils/cn";
import toCSS from "@/utils/toCSS";
import { animated, useSpring } from "@react-spring/web";

import styles from "./circular.module.css";

import { type PropsWithChildren, useCallback, useRef } from "react";

export interface BolderHoldCircularProps extends PropsWithChildren {
    // holdDuration?: number;
    // holdFactor?: number;
    // returnSpeed?: number;
    onHold?: () => void;
}

export default function BorderHoldCircular({ children, onHold }: BolderHoldCircularProps) {
    const wrapperRef = useRef<HTMLDivElement>(null);

    const { width, height } = useSize(() => wrapperRef.current) ?? {
        width: 0,
        height: 0,
    };

    const bgWidth = width * (1 + (1 / 15));
    const bgHeight = height * (1 + (1 / 15));

    const [{ opacity }, opacityApi] = useSpring(() => ({
        opacity: 0,
    }));

    const dispatched = useRef(false);

    const [{ progress }, api] = useSpring(() => ({
        progress: 0,
        config: {
            mass: 5,
            friction: 110,
            tension: 170,
        },
        onStart() {
            opacityApi.start({
                opacity: 1,
            });
        },
        onChange(foo) {
            if (foo.value.progress > 98) {
                if (dispatched.current)
                    return;
                dispatched.current = true;

                onHold?.();
            } else if (foo.value.progress < 2) {
                if (!dispatched.current)
                    return;
                dispatched.current = false;
                api.stop();
                opacityApi.start({
                    opacity: 0,
                    onRest() {
                    },
                });
            }
        },
    }));


    const startAnimation = useCallback(() => {
        api.start({
            progress: 100,
        });
    }, [api]);

    const stopAnimation = useCallback(() => {
        api.start({
            progress: 0,
        });
    }, [api]);

    return (
        <div
            ref={wrapperRef}
            onMouseDown={startAnimation}
            onMouseUp={stopAnimation}
            onMouseOut={stopAnimation}
        >
            <animated.svg
                className={cn("absolute -translate-1/30 -z-10", styles.circularBorder)}
                viewBox="0 0 250 250"
                style={{
                    width: toCSS.px(bgWidth),
                    height: toCSS.px(bgHeight),
                    ["--border-hold-progress" as any]: progress,
                    opacity,
                }}
            >
                <circle
                    className={cn("rounded-full w-full h-full")}
                />
            </animated.svg>
            {children}
        </div>
    );
}
