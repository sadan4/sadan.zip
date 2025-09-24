import { useControlledState } from "@/hooks/controlledState";
import cn from "@/utils/cn";
import { animated, useTransition } from "@react-spring/web";

import { TooltipPosition } from "./constants";
import styles from "./styles.module.scss";
import { Box } from "../layout/Box";

import type { PropsWithChildren, ReactNode } from "react";

export interface TooltipProps extends PropsWithChildren {
    /**
     * The content of the tooltip
     */
    text: ReactNode;
    /**
     * The position of the tooltip, leave blank for default (TOP)
     */
    position?: TooltipPosition;
    show?: boolean;
    onShow?(): void;
    onHide?(): void;
    className?: string;
}

function useTooltipAnim(shouldShow: boolean, position: TooltipPosition) {
    return useTransition(shouldShow, {
        from: {
            opacity: 0,
            scale: 0.7,
            top: -80,
        },
        enter: {
            opacity: 1,
            scale: 1,
            top: -100,
        },
        leave: {
            opacity: 0,
            scale: 0.7,
            top: -120,
        },
    });
}

export function Tooltip({
    text,
    show: _show,
    onShow,
    onHide,
    className,
    position = TooltipPosition.TOP,
    children,
}: TooltipProps) {
    const [shouldShow, setShouldShow] = useControlledState({
        initialValue: false,
        managedValue: _show,
        handleChange: (s) => (s ? onShow : onHide)?.(),
        debugName: "Tooltip",
    });

    const tooltipTransition = useTooltipAnim(shouldShow, position);
    const show = () => setShouldShow(true);
    const hide = () => setShouldShow(false);

    return (
        <div
            className={cn(styles.tooltip, className)}
            onMouseOver={show}
            onMouseOut={hide}
        >
            {
                tooltipTransition(({ opacity, scale, top }, show) => show && (
                    <animated.div
                        className={cn(styles.wrapper)}
                        style={{
                            opacity,
                            scale,
                            top: top.to((t) => `${t}%`),
                        }}
                    >
                        <Box>
                            {text}
                        </Box>
                    </animated.div>
                ))
            }
            <div
                className={cn(styles.trigger)}
            >
                {children}
            </div>
        </div>
    );
}
