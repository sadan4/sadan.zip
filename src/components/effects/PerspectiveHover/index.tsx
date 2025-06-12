import { useHover } from "@/hooks/hover";
import { useSize } from "@/hooks/size";
import cn from "@/utils/cn";

import styles from "./style.module.css";

import { type CSSProperties, type JSX, type MouseEventHandler, useCallback, useEffect, useMemo, useRef, useState } from "react";

export interface PerspectiveHoverProps {
    children: (props: {
        style?: CSSProperties;
        className?: string;
    }) => JSX.Element;
    className?: string;
    shineClassName?: string;
    noShine?: boolean;
}

// https://codepen.io/robin-dela/pen/jVddbq
export default function PerspectiveHover({ children: Children, className = "", shineClassName = "", noShine = false }: PerspectiveHoverProps) {
    type TMouseEvent = MouseEventHandler<HTMLDivElement>;

    const wrapperRef = useRef<HTMLDivElement>(null);
    const [contentStyle, setContentStyle] = useState<CSSProperties>();
    const [shineStyle, setShineStyle] = useState<CSSProperties>({});
    const [wrapperStyle, setWrapperStyle] = useState<CSSProperties | undefined>();
    const [shadowStyle, setShadowStyle] = useState<CSSProperties>();
    const hover = useHover(wrapperRef);

    function makeDefaultWrapperStyle() {
        const { clientWidth, offsetWidth, scrollWidth } = wrapperRef.current!;
        const w = clientWidth || offsetWidth || scrollWidth;

        return {
            transform: `perspective(${w * 3}px)`,
        };
    }

    const [defaultWrapperStyle, setDefaultWrapperStyle] = useState<CSSProperties>(null!);

    useEffect(() => {
        const defaultStyle = makeDefaultWrapperStyle();

        setDefaultWrapperStyle(defaultStyle);
        setWrapperStyle((old) => {
            return {
                ...old,
                ...defaultStyle,
            };
        });
    }, []);

    const { width: shadowWidth, height: shadowHeight } = useSize(wrapperRef.current) ?? {
        width: 0,
        height: 0,
    };

    useEffect(() => {
        setShadowStyle((old) => {
            const base = Math.log(shadowHeight);

            return {
                ...old,
                "--container-width": `${shadowWidth}px`,
                "--container-height": `${shadowHeight}px`,
                "--log-container-height": `${base}px`,
            };
        });
    }, [shadowWidth, shadowHeight]);

    const handleMove = useCallback<TMouseEvent>((ev) => {
        const { pageX, pageY } = ev;
        const elem = wrapperRef.current!;
        const bodyTop = document.body.scrollTop || document.documentElement.scrollTop;
        const bodyLeft = document.body.scrollLeft;
        const offsets = elem.getBoundingClientRect();
        const w = elem.clientWidth || elem.offsetWidth || elem.scrollWidth;
        const h = elem.clientHeight || elem.offsetHeight || elem.scrollHeight;
        const offsetX = 0.52 - ((pageX - offsets.left - bodyLeft) / w);
        const offsetY = 0.52 - ((pageY - offsets.top - bodyTop) / h);
        const dy = pageY - offsets.top - bodyTop - (h / 2);
        const dx = pageX - offsets.left - bodyLeft - (w / 2);
        const yRotate = (offsetX - dx) * 0.14;
        const xRotate = (dy - offsetY) * 0.2;
        const arad = Math.atan2(dy, dx);
        let angle = ((arad * 180) / Math.PI) - 90;

        if (angle < 0) {
            angle += 360;
        }
        setWrapperStyle((old) => {
            return {
                ...old,
                transform: `rotateX(${xRotate}deg) rotateY(${yRotate}deg) ${hover ? "scale3d(1.07, 1.07, 1.07)" : ""}`,
            };
        });
        setContentStyle((old) => {
            return {
                ...old,
                transform: `translateX(${offsetX * 2.5}px) translateY(${offsetY * 2.5}px)`,
            };
        });
        if (noShine)
            return;
        setShineStyle((old) => {
            return {
                ...old,
                background: `linear-gradient(${angle}deg, rgba(255,255,255,${((pageY - offsets.top - bodyTop) / h) * 0.4}) 0%, rgba(255,255,255,0) 80%)`,
                transform: `translateX(${offsetX - 0.1}px) translateY(${offsetY - 0.1}px)`,
            };
        });
    }, [hover, noShine]);

    const handleLeave = useCallback<TMouseEvent>(() => {
        setWrapperStyle((old) => {
            return {
                ...old,
                transform: undefined,
                ...defaultWrapperStyle,
            };
        });
        setShineStyle({});
        setContentStyle((old) => {
            return {
                ...old,
                transform: undefined,
            };
        });
    }, [defaultWrapperStyle]);

    const renderedChildren = useMemo(() => {
        return (
            <>
                <Children
                    style={{
                        ...shadowStyle,
                    }}
                    className={cn(
                        styles.dropShadow,
                        hover && styles.hovered,
                    )}
                />
            </>
        );
    }, [Children, hover, shadowStyle]);

    return (
        <div
            ref={wrapperRef}
            style={wrapperStyle}
            className={`${className}`}
            onMouseLeave={handleLeave}
            onMouseMove={handleMove}
        >
            {!noShine && (
                <div
                    className={`${shineClassName} z-10 absolute top-0 bottom-0 left-0 right-0 pointer-events-none`}
                    style={shineStyle}
                />
            )}
            <div
                className="-z-10 w-auto h-auto flex"
                style={contentStyle}
            >
                {renderedChildren}
            </div>
        </div>
    );
}
