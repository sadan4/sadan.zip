import { useEventHandler } from "@/hooks/eventListener";
import cn from "@/utils/cn";
import { useCssFile } from "@/utils/cssFile";

import noCursorStyle from "./style.css?url";

import { type CSSProperties, type PropsWithChildren, type ReactNode, useCallback, useEffect, useRef, useState } from "react";

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

export interface AnimatedCursorProps extends AnimatedCursorOptions, PropsWithChildren {
    className?: string;
    clickables?: Clickable[];
    showSystemCursor?: boolean;
    trailingSpeed?: number;
}

export interface AnimatedCursorCoordinates {
    x: number;
    y: number;
}

export default function Cursor({ className = "", children }: AnimatedCursorProps) {
    useCssFile(noCursorStyle);

    const [coords, setCoords] = useState<AnimatedCursorCoordinates>({
        x: 0,
        y: 0,
    });

    const [down, setDown] = useState(false);
    const [visible, setVisible] = useState(false);
    const cursorRef = useRef<HTMLDivElement>(null);

    const onMouseMove = useCallback((ev: MouseEvent) => {
        const { clientX, clientY } = ev;

        setCoords({
            x: clientX,
            y: clientY,
        });
        if (cursorRef.current) {
            cursorRef.current.style.top = `${clientY}px`;
            cursorRef.current.style.left = `${clientX}px`;
        }
    }, []);

    const onMouseDown = useCallback(() => {
        setDown(true);
    }, []);

    const onMouseUp = useCallback(() => {
        setDown(false);
    }, []);

    const onMouseEnterViewport = useCallback(() => {
        setVisible(true);
    }, []);

    const onMouseLeaveViewport = useCallback(() => {
        setVisible(false);
    }, []);

    useEventHandler("mousemove", onMouseMove);
    useEventHandler("mousedown", onMouseDown);
    useEventHandler("mouseup", onMouseUp);
    useEventHandler("mouseover", onMouseEnterViewport);
    useEventHandler("mouseout", onMouseLeaveViewport);

    useEffect(() => {
        if (!cursorRef.current)
            return;

        cursorRef.current.style.opacity = `${+visible}`;
    }, [visible]);

    return (
        <div
            className={cn(className, "fixed z-99999 pointer-events-none")}
            ref={cursorRef}
        >
            {children}
        </div>
    );
}
