import { useCssFile } from "@/hooks/cssFile";
import { useEventHandler } from "@/hooks/eventListener";
import cn from "@/utils/cn";
import { disposableEventHandler } from "@/utils/events";

import { useCursorContextStore } from "./cursorContextStore";
import noCursorStyle from "./hideNativeCursor.css?url";
import styles from "./styles.module.css";

import { type CSSProperties, type ReactNode, useCallback, useEffect, useRef } from "react";

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

    useEffect(() => {
        const clickableElements = document.querySelectorAll("[data-clickable]");
        const cleanups: (() => void)[] = [];

        for (const el of clickableElements) {
            cleanups.push(disposableEventHandler("mouseover", () => {
                useCursorContextStore.getState()
                    .updateClickableElement(el);
            }, el as HTMLElement));
            cleanups.push(disposableEventHandler("mouseout", () => {
                useCursorContextStore.getState()
                    .updateClickableElement(null);
            }, el as HTMLElement));
        }

        return () => {
            for (const cleanup of cleanups) {
                cleanup();
            }
        };
    });

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
