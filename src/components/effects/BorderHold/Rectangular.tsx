import { useSize } from "@/hooks/size";
import toCSS from "@/utils/toCSS";
import { animated, useSpringValue } from "@react-spring/web";

import styles from "./rectangular.module.scss";

import { type PropsWithChildren, useCallback, useRef } from "react";

export interface BolderHoldCircularProps extends PropsWithChildren {
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
    const opacity = useSpringValue(0);
    const dispatched = useRef(false);

    const progress = useSpringValue(0, {
        config: {
            mass: 5,
            friction: 110,
        },
        onChange(_foo) {
            // https://github.com/pmndrs/react-spring/issues/2183
            const foo: number = typeof _foo === "number"
                ? _foo
                : _foo.value;

            if (foo > 98 && progress.goal === 100 && !dispatched.current) {
                dispatched.current = true;
                onHold?.();
            } else if (foo < 2 && progress.goal === 0) {
                opacity.start(0);
                dispatched.current = false;
            }
        },
    });

    const startAnimation = useCallback(() => {
        progress.start(100, {
            config: {
                friction: 110,
            },
        });
        opacity.start(1);
    }, [opacity, progress]);

    const stopAnimation = useCallback(() => {
        progress.start(0, {
            config: {
                friction: 55,
            },
        });
    }, [progress]);

    return (
        <div
            className="relative"
            onPointerDown={startAnimation}
            onContextMenu={(e) => {
                // it's a pointer event, react is stupid https://developer.mozilla.org/en-US/docs/Web/API/Element/contextmenu_event#browser_compatibility
                if ((e.nativeEvent as PointerEvent).pointerType !== "mouse") {
                    e.preventDefault();
                }
            }}
            onPointerUp={stopAnimation}
            onPointerLeave={stopAnimation}
        >
            <div
                className="contents"
                ref={wrapperRef}
            >
                {children}
            </div>
            <animated.svg
                className={styles.rectBorder}
                viewBox="0 0 250 250"
                style={{
                    width: toCSS.px(bgWidth),
                    height: toCSS.px(bgHeight),
                    ["--border-hold-progress" as any]: progress,
                    opacity,
                }}
            >
                <rect />
            </animated.svg>
        </div>
    );
}
