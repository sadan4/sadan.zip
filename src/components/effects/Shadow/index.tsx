import { useRect } from "@/hooks/rect";
import cn from "@/utils/cn";

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

        // oxlint-disable-next-line react/react-compiler
        setCssProps({
            "--shadow-container-width": `${width}px`,
            "--shadow-container-height": `${height}px`,
            "--log-shadow-container-height": `${logHeight}px`,
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
