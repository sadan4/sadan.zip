import { useSize } from "@/hooks/size";
import cn from "@/utils/cn";
import toCSS from "@/utils/toCSS";

import styles from "./style.module.scss";

import { type CSSProperties, type PropsWithChildren, useLayoutEffect, useRef, useState } from "react";

export interface ShadowProps extends PropsWithChildren {
    noHover?: boolean;
}

export default function Shadow({ children, noHover = false }: ShadowProps) {
    const ref = useRef<HTMLDivElement>(null);

    const { width, height } = useSize(() => ref.current) ?? {
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
            ref={ref}
            style={cssProps}
            className={cn(noHover ? styles.dropShadowNoHover : styles.dropShadow)}
        >
            {children}
        </div>
    );
}
