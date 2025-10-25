import { useRect } from "@/hooks/rect";
import cn from "@/utils/cn";
import toCSS from "@/utils/toCSS";
import { animated } from "@react-spring/web";

import styles from "./circular.module.scss";
import { type BaseBorderHoldProps, useBorderHoldAnim } from "./common";

import { useState } from "react";

export interface BorderHoldCircularProps extends BaseBorderHoldProps {
}

export function BorderHoldCircular({ children, onHold }: BorderHoldCircularProps) {
    const [wrapper, setWrapper] = useState<HTMLDivElement | null>(null);
    const [held, setHeld] = useState(false);

    const { width, height } = useRect(wrapper) ?? {
        width: 0,
        height: 0,
    };

    const bgWidth = width * (1 + (1 / 15));
    const bgHeight = height * (1 + (1 / 15));

    const { progress, opacity } = useBorderHoldAnim({
        held,
        onHold,
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
                ref={setWrapper}
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
