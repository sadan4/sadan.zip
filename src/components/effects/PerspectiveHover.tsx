import { type CSSProperties, type MouseEventHandler, type PropsWithChildren, useCallback, useEffect, useRef, useState } from "react";

export interface ShineProps extends PropsWithChildren {
    className?: string;
}

// https://codepen.io/robin-dela/pen/jVddbq
export default function PerspectiveHover({ children, className = "" }: ShineProps) {
    type TMouseEvent = MouseEventHandler<HTMLDivElement>;

    const wrapperRef = useRef<HTMLDivElement>(null);
    const [shineStyle, setShineStyle] = useState<CSSProperties | undefined>();
    const [contentStyle, setContentStyle] = useState<CSSProperties>();
    const [wrapperStyle, setWrapperStyle] = useState<CSSProperties | undefined>();
    const [hover, setHover] = useState(false);

    function makeDefaultWrapperStyle() {
        const { clientWidth, offsetWidth, scrollWidth } = wrapperRef.current!;
        const w = clientWidth || offsetWidth || scrollWidth;

        return {
            transform: `perspective(${w * 3}px)`,
        };
    }

    const [defaultWrapperStyle, setDefaultWrapperStyle] = useState<CSSProperties>(null!);

    useEffect(() => {
        setHover(wrapperRef.current!.matches(":hover"));

        const defaultStyle = makeDefaultWrapperStyle();

        setDefaultWrapperStyle(defaultStyle);
        setWrapperStyle((old) => {
            return {
                ...old,
                ...defaultStyle,
            };
        });
    }, []);

    const handleEnter = useCallback<TMouseEvent>(() => {
        setHover(true);
    }, []);

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
        const yRotate = (offsetX - dx) * 0.07;
        const xRotate = (dy - offsetY) * 0.1;
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
        setShineStyle((old) => {
            return {
                ...old,
                background: `linear-gradient(${angle}deg, rgba(255,255,255,${((pageY - offsets.top - bodyTop) / h) * 0.4}) 0%, rgba(255,255,255,0) 80%)`,
                transform: `translateX(${offsetX - 0.1}px) translateY(${offsetY - 0.1}px)`,
            };
        });
        setContentStyle((old) => {
            return {
                ...old,
                transform: `translateX(${offsetX * 2.5}px) translateY(${offsetY * 2.5}px)`,
            };
        });
    }, [hover]);

    const handleLeave = useCallback<TMouseEvent>(() => {
        setHover(false);
        setWrapperStyle((old) => {
            return {
                ...old,
                transform: undefined,
                ...defaultWrapperStyle,
            };
        });
        setShineStyle(void 0);
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
            className={className}
            onMouseEnter={handleEnter}
            onMouseLeave={handleLeave}
            onMouseMove={handleMove}
        >
            <div
                className="z-10 absolute top-0 bottom-0 left-0 right-0 rounded-lg pointer-events-none"
                style={shineStyle}
            />
            <div
                className="-z-10"
                style={contentStyle}
            >
                {children}
            </div>
        </div>
    );
}
