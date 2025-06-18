import { useCursorVisible } from "@/hooks/cursorVisible";
import { useCursorOpacityStore } from "@/stores/cursorOpacity";
import cn from "@/utils/cn";
import { animated, useSpring } from "@react-spring/web";

import styles from "./DotCursor.module.css";

import { useEffect } from "react";


export interface DotCursorProps {
    className?: string;
    radius?: number;
    invert?: boolean;
}

export function DotCursor({ className, radius = 6, invert }: DotCursorProps) {
    const cursorVisible = useCursorVisible();

    const [{ opacity }, api] = useSpring(() => ({
        opacity: 1,
    }));

    const storeOpacity = useCursorOpacityStore((state) => state.currentOpacity);

    useEffect(() => {
        api.start({ opacity: storeOpacity });
    }, [storeOpacity, api]);

    return cursorVisible && (
        <animated.div
            className={cn("rounded-full", className, invert && styles.invertCursor)}
            style={{
                width: `${radius}px`,
                height: `${radius}px`,
                opacity,
            }}
        />
    );
}

