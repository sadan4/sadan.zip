import { useCssFile } from "@/hooks/cssFile";
import { useEventHandler } from "@/hooks/eventListener";
import cn from "@/utils/cn";

import noCursorStyle from "./hideNativeCursor.css?url";
import styles from "./styles.module.css";

import { type CSSProperties, type ReactNode, useCallback, useRef } from "react";

export interface AnimatedCursorOptions {
    children?: ReactNode;
    color?: string;
    innerScale?: number;
    innerSize?: number;
    innerStyle?: CSSProperties;
    outerAlpha?: number;
    outerScale?: number;
    outerSize?: number;
    outerStyle?: CSSProperties;
}

export type Clickable = string | ({ target: string; } & AnimatedCursorOptions);

export interface AnimatedCursorProps extends AnimatedCursorOptions {
    className?: string;
    clickables?: Clickable[];
    showSystemCursor?: boolean;
    trailingSpeed?: number;
}

export default function Cursor({ className = "", children }: AnimatedCursorProps) {
    useCssFile(noCursorStyle);

    const cursorRef = useRef<HTMLDivElement>(null);

    const onMouseMove = useCallback((ev: MouseEvent) => {
        const { clientX, clientY } = ev;

        if (cursorRef.current) {
            cursorRef.current.style.top = `${clientY}px`;
            cursorRef.current.style.left = `${clientX}px`;
        }
    }, []);

    useEventHandler("mousemove", onMouseMove);

    return (
        <div
            className={cn(styles.cursorZ, "fixed pointer-events-none -translate-1/2", className)}
            ref={cursorRef}
        >
            {children}
        </div>
    );
}
