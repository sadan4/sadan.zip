import { useCursorVisible } from "@/hooks/cursorVisible";
import cn from "@/utils/cn";
import { getLineHeight } from "@/utils/dom";
import { animated, useSpring } from "@react-spring/web";

import { useCursorContextStore } from "./cursorContextStore";
import styles from "./DotCursor.module.css";

import { useEffect } from "react";
import { useShallow } from "zustand/react/shallow";


export interface DotCursorProps {
    className?: string;
    radius?: number;
    invert?: boolean;
    lineOnText?: boolean;
}

export function DotCursor({ className, radius = 6, invert, lineOnText = false }: DotCursorProps) {
    const cursorVisible = useCursorVisible();

    const { mouseDown, textElement } = useCursorContextStore(useShallow((s) => ({
        mouseDown: s.mouseDown,
        textElement: s.textElement,
    })));

    console.log(`mouseDown: ${mouseDown}`);

    const [{ height, width }, sizeApi] = useSpring(() => ({
        height: radius,
        width: radius,
        config: {
            mass: 2,
        },
    }));

    useEffect(() => {
        if (!lineOnText) {
            return;
        }
        if (textElement != null) {
            sizeApi.start({
                height: getLineHeight(textElement) * 0.8,
                width: radius / (mouseDown ? 4 : 2),
            });
        } else {
            sizeApi.start({
                height: radius,
                width: radius,
            });
        }
    }, [radius, sizeApi, textElement, mouseDown, lineOnText]);


    return cursorVisible && (
        <animated.div
            className={cn("rounded-full", className, invert && styles.invertCursor)}
            style={{
                width,
                height,
            }}
        />
    );
}

