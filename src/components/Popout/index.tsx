import cn from "@/utils/cn";
import toCSS from "@/utils/toCSS";
import * as Popover from "@radix-ui/react-popover";

import { PopoutDirection } from "./constants";
import styles from "./style.module.css";

import { type PropsWithChildren, type ReactNode, useCallback, useEffect, useRef, useState } from "react";

export interface RenderPopoutProps {
    close(): void;
}

export interface PopoutProps extends PropsWithChildren {
    open?: boolean;
    onClose?: () => void;
    renderPopout(props: RenderPopoutProps): ReactNode;
    openOnClick?: boolean;
    side: PopoutDirection;
}

export function Popout({ open, children, side, renderPopout: RenderPopout }: PopoutProps) {
    const isCenter = side === PopoutDirection.CENTER;
    const [popoutOpen, setPopoutOpen] = useState(open ?? false);

    useEffect(() => {
        if (open != null) {
            setPopoutOpen(open);
        }
    }, [open]);

    const onOpenChange = useCallback((state: boolean) => {
        setPopoutOpen(state);
    }, []);

    const closePopout = useCallback(() => {
        setPopoutOpen(false);
    }, []);

    const contentRef = useRef<HTMLDivElement>(null);
    const [triggerWidth, setTriggerWidth] = useState(0);
    const [triggerHight, setTriggerHight] = useState(0);

    useEffect(() => {
        if (!contentRef.current)
            return;

        const { width, height } = contentRef.current.getBoundingClientRect();

        setTriggerWidth(width);
        setTriggerHight(height);
    }, []);

    return (
        <Popover.Root
            open={popoutOpen}
            onOpenChange={onOpenChange}
        >
            <Popover.Trigger asChild>
                <div
                    ref={contentRef}
                    onClick={(ev) => {
                        ev.stopPropagation();
                    }}
                >
                    {children}
                </div>
            </Popover.Trigger>
            <div
                className={cn(styles.popoutContentWrapper)}
                style={{
                    ["--trigger-width" as any]: toCSS.px(triggerWidth),
                    ["--trigger-height" as any]: toCSS.px(triggerHight),
                }}
                data-center={isCenter}
            >
                <Popover.Content
                    side={isCenter ? "bottom" : side}
                    asChild
                >
                    <div
                        className="foobar"
                        style={isCenter
                            ? {
                                transform: toCSS.translate("-50%", "-50%"),
                            }
                            : {}}
                    >
                        <RenderPopout close={closePopout} />
                    </div>
                </Popover.Content>
            </div>
        </Popover.Root>
    );
}
