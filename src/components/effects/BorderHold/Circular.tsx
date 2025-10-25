import { useSize } from "@/hooks/size";
import cn from "@/utils/cn";
import { unreachable } from "@/utils/error";
import toCSS from "@/utils/toCSS";
import { animated, type SpringConfig, useSpring } from "@react-spring/web";

import styles from "./circular.module.scss";
import type { BaseBorderHoldProps } from "./common";

import { useRef, useState } from "react";

export interface BorderHoldCircularProps extends BaseBorderHoldProps {
}

export function BorderHoldCircular({ children, onHold }: BorderHoldCircularProps) {
    const wrapperRef = useRef<HTMLDivElement>(null);
    const [held, setHeld] = useState(false);

    const { width, height } = useSize(() => wrapperRef.current) ?? {
        width: 0,
        height: 0,
    };

    const bgWidth = width * (1 + (1 / 15));
    const bgHeight = height * (1 + (1 / 15));
    // const opacity = useSpringValue(0);
    const dispatched = useRef(false);

    const { progress, opacity } = useSpring({
        from: {
            progress: 0,
            opacity: 0,
        },
        async to(next) {
            if (held) {
                await next({
                    progress: 100,
                    opacity: 1,
                    onChange(progress) {
                        if (!progress.cancelled && !dispatched.current && (progress.value.progress as number) >= 98) {
                            dispatched.current = true;
                            onHold?.();
                        }
                    },
                });
            } else {
                await next({
                    progress: 0,
                    onChange(progress) {
                        if (!progress.cancelled && (progress.value.progress as number) <= 5) {
                            // react spring doesn't like this, but it works
                            next({
                                opacity: 0,
                            }).catch(() => {});
                            dispatched.current = false;
                        }
                    },
                });
            }
        },
        config(k): SpringConfig {
            switch (k as "opacity" | "progress") {
                case "opacity":
                    return {};
                case "progress":
                    if (held) {
                        return {
                            mass: 5,
                            friction: 110,
                        };
                    }
                    return {
                        mass: 5,
                        friction: 50,
                    };

                default:
                    unreachable();
            }
        },
    });

    return (
        <div
            className="relative"
            onPointerDown={() => {
                setHeld(true);
            }}
            onContextMenu={(e) => {
                // it's a pointer event, react is stupid https://developer.mozilla.org/en-US/docs/Web/API/Element/contextmenu_event#browser_compatibility
                if ((e.nativeEvent as PointerEvent).pointerType !== "mouse") {
                    e.preventDefault();
                }
            }}
            onPointerUp={() => {
                setHeld(false);
            }}
            onPointerLeave={() => {
                setHeld(false);
            }}
        >
            <div
                className="contents"
                ref={wrapperRef}
            >
                {children}
            </div>
            <animated.svg
                className={styles.circularBorder}
                viewBox="0 0 250 250"
                style={{
                    width: toCSS.px(bgWidth),
                    height: toCSS.px(bgHeight),
                    "--border-hold-progress": progress,
                    opacity,
                }}
            >
                <circle
                    className={cn("h-full w-full rounded-full")}
                />
            </animated.svg>
        </div>
    );
}
