import { type CSSProperties, type MouseEventHandler, type PropsWithChildren, useCallback, useEffect, useRef, useState } from "react";

export interface PerspectiveHoverProps extends PropsWithChildren {

}

// https://codepen.io/robin-dela/pen/jVddbq
export default function PerspectiveHover({ children }: PerspectiveHoverProps) {
    type TMouseEvent = MouseEventHandler<HTMLDivElement>;

    const wrapperRef = useRef<HTMLDivElement>(null);
    const [contentStyle, setContentStyle] = useState<CSSProperties>();
    const [wrapperStyle, setWrapperStyle] = useState<CSSProperties | undefined>();

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

    const handleMove = useCallback<TMouseEvent>((ev) => {
        const { pageX, pageY } = ev;
        const elem = wrapperRef.current!;
        const bodyTop = document.body.scrollTop || document.documentElement.scrollTop;
        const bodyLeft = document.body.scrollLeft;
        const offsets = elem.getBoundingClientRect();
        const wrapperWidth = elem.clientWidth || elem.offsetWidth || elem.scrollWidth;
        const wrapperHeight = elem.clientHeight || elem.offsetHeight || elem.scrollHeight;
        const offsetX = 0.52 - ((pageX - offsets.left - bodyLeft) / wrapperWidth);
        const offsetY = 0.52 - ((pageY - offsets.top - bodyTop) / wrapperHeight);
        const dy = pageY - offsets.top - bodyTop - (wrapperHeight / 2);
        const dx = pageX - offsets.left - bodyLeft - (wrapperWidth / 2);
        const yRotate = (offsetX - dx) * 0.07;
        const xRotate = (dy - offsetY) * 0.1;

        setWrapperStyle((old) => {
            return {
                ...old,
                transform: `rotateX(${xRotate}deg) rotateY(${yRotate}deg)`,
            };
        });
        setContentStyle((old) => {
            return {
                ...old,
                transform: `translateX(${offsetX * 2.5}px) translateY(${offsetY * 2.5}px)`,
            };
        });
    }, []);

    const handleLeave = useCallback<TMouseEvent>(() => {
        setWrapperStyle((old) => {
            return {
                ...old,
                transform: undefined,
                ...defaultWrapperStyle,
            };
        });
        setContentStyle((old) => {
            return {
                ...old,
                transform: undefined,
            };
        });
    }, [defaultWrapperStyle]);

    return (
        <div
            ref={wrapperRef}
            style={wrapperStyle}
            onMouseLeave={handleLeave}
            onMouseMove={handleMove}
        >
            <div
                style={contentStyle}
            >
                {children}
            </div>
        </div>
    );
}
