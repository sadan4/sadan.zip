import { unreachable } from "@/utils/error";
import { type SpringConfig, useSpring } from "@react-spring/web";

import { type PropsWithChildren, useRef } from "react";

export interface BaseBorderHoldProps extends PropsWithChildren {
    onHold?: () => void;
}

declare module "react" {
    interface CSSProperties {
        "--border-hold-progress"?: number;
    }
}

export interface UseBorderHoldAnimProps {
    onHold?(): void;
    held: boolean;
}

export function useBorderHoldAnim({ onHold, held }: UseBorderHoldAnimProps) {
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

    return {
        progress,
        opacity,
    };
}
