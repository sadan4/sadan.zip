import { useRect } from "@/hooks/rect";
import cn from "@/utils/cn";
import toCSS from "@/utils/toCSS";

import * as styles from "./style.module.scss";

import { type CSSProperties, type PropsWithChildren, useLayoutEffect, useState } from "react";

export interface ShadowProps extends PropsWithChildren {
    noHover?: boolean;
    className?: string;
}

export default function Shadow({ children, noHover = false, className }: ShadowProps) {
    const [el, setEl] = useState<HTMLDivElement | null>(null);

    const { width, height } = useRect(el) ?? {
        width: 0,
        height: 0,
    };

    const [cssProps, setCssProps] = useState<CSSProperties>({});

    useLayoutEffect(() => {
        const logHeight = Math.log(height);

        setCssProps({
            "--shadow-container-width": toCSS.px(width),
            "--shadow-container-height": toCSS.px(height),
            "--log-shadow-container-height": toCSS.px(logHeight),

        } as CSSProperties);
    }, [width, height]);

    return (
        <div
            ref={setEl}
            style={cssProps}
            className={cn(noHover ? styles.dropShadowNoHover : styles.dropShadow, className)}
        >
            {children}
        </div>
    );
}
