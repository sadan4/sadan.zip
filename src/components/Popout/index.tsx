import { useControlledState } from "@/hooks/controlledState";
import { useRect } from "@/hooks/rect";
import cn from "@/utils/cn";
import { error } from "@/utils/error";

import { Position } from "./enums";
import styles from "./styles.module.scss";
import { Clickable } from "../Clickable";

import { createContext, type PropsWithChildren, use, useEffect, useMemo, useRef, useState } from "react";

export interface PopoutRootProps extends PropsWithChildren {
    open?: boolean;
    onClose?(): void;
    onOpen?(): void;
}

export interface PopoutTriggerProps extends PropsWithChildren {
}

export interface PopoutContentProps extends PropsWithChildren {
    position?: Position;
    onDismiss?(): void;
}

declare module "react" {
    interface CSSProperties {
        "--pop-top"?: number;
        "--pop-left"?: number;
        "--pop-width"?: number;
        "--pop-height"?: number;
    }
}

interface InternalPopoutContext {
    open(): void;
    close(): void;
    isOpen: boolean;
    setRect(rect: DOMRectReadOnly | undefined): void;
    rect?: DOMRectReadOnly;
}

const InternalPopoutContext = createContext<InternalPopoutContext | null>(null);

InternalPopoutContext.displayName = "InternalPopoutContext";

function usePopoutContextInternal(): InternalPopoutContext {
    const ctx = use(InternalPopoutContext);

    if (ctx == null) {
        error("usePopoutContextInternal must be used within a Popout2.Root");
    }
    return ctx;
}

const positionMap: Record<Position, string> = {
    [Position.TOP]: styles.top,
    [Position.BOTTOM]: styles.bottom,
    [Position.LEFT]: styles.left,
    [Position.RIGHT]: styles.right,
    [Position.CENTER]: styles.center,
};

export function PopoutRoot({ children, open: _value, onClose, onOpen }: PopoutRootProps) {
    const [isOpen, setIsOpen] = useControlledState({
        initialValue: false,
        debugName: "Popout2.Open",
        managedValue: _value,
        handleChange(open) {
            if (open) {
                onOpen?.();
            } else {
                onClose?.();
            }
        },
    });

    const [rect, setRect] = useState<DOMRectReadOnly>();

    const api = useMemo<InternalPopoutContext>(() => ({
        open() {
            setIsOpen(true);
        },
        close() {
            setIsOpen(false);
        },
        setRect,
        rect,
        isOpen,
    }), [isOpen, rect, setIsOpen]);

    return (
        <InternalPopoutContext value={api}>
            {children}
        </InternalPopoutContext>
    );
}

export function PopoutTrigger({ children }: PopoutTriggerProps) {
    const ctx = usePopoutContextInternal();
    const [el, setEl] = useState<HTMLElement | null>(null);
    const rect = useRect(el);

    useEffect(() => {
        ctx.setRect(rect);
    }, [rect, ctx]);

    return (
        <Clickable
            ref={setEl}
            className="contents"
            onClick={ctx.open}
        >
            {children}
        </Clickable>
    );
}

export function PopoutContent({ children, position = Position.TOP, onDismiss }: PopoutContentProps) {
    const ctx = usePopoutContextInternal();
    const ref = useRef<HTMLDivElement>(null);

    useEffect(() => {
        if (ctx.isOpen) {
            ref.current?.showPopover();
        } else {
            ref.current?.hidePopover();
        }
    }, [ctx.isOpen]);

    return (
        <div
            ref={ref}
            popover="auto"
            style={{
                "--pop-top": ctx.rect?.top,
                "--pop-left": ctx.rect?.left,
                "--pop-width": ctx.rect?.width,
                "--pop-height": ctx.rect?.height,
            }}
        >
            <div
                className="fixed inset-fill"
                onClick={() => {
                    onDismiss?.();
                    ctx.close();
                }}
            >
                <div
                    className={cn(styles.content, positionMap[position])}
                    onClick={(e) => e.stopPropagation()}
                >
                    {children}
                </div>
            </div>
        </div>
    );
}
