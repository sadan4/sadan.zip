import { useKeybinds } from "@/hooks/keybind";
import cn from "@/utils/cn";
import { updateRef } from "@/utils/ref";
import { animated, useTransition } from "@react-spring/web";

import { SYM_INTERNAL_KEY, useModalStackStore } from "./internal/modalStackStore";
import { ModalContext } from "./context";
import { exitModalKeybinds, type IModalContext, type Modal } from ".";
import { LayerPortal } from "../Layer";

import { type BaseSyntheticEvent, useCallback, useEffect, useMemo, useRef } from "react";


function stopParentPropagation(ev: BaseSyntheticEvent) {
    ev.stopPropagation();
}

function makeContext(modal: Modal | undefined): IModalContext | undefined {
    return modal && {
        ...modal,
        close() {
            useModalStackStore.getState()._popModalByInternalKey(modal[SYM_INTERNAL_KEY]);
        },
    };
}

function Render({ context }: { context?: IModalContext; }) {
    if (!context)
        return null;

    return context.Render.apply(context);
}

export default function ModalRenderRoot() {
    const currentModal = useModalStackStore((state) => state.modals.at(-1));
    const modalContext = useMemo(() => makeContext(currentModal), [currentModal]);

    const transitions = useTransition(modalContext, {
        from: { opacity: 0 },
        enter: { opacity: 1 },
        leave: { opacity: 0 },
        config: {
            tension: 300,
            friction: 25,
        },
    });

    const closing = useRef(false);
    const ref = useRef<HTMLDivElement>(null);

    const forceCloseModal = useCallback(() => {
        try {
            if (!currentModal)
                return;
            currentModal.onModalClose?.apply(modalContext!);
            useModalStackStore.getState()
                ._popModalByInternalKey(currentModal[SYM_INTERNAL_KEY]);
        } catch (e) {
            console.log("Failed to close modal", e);
            throw e;
        }
    }, [currentModal, modalContext]);

    const closeModal = useCallback(async () => {
        if (!currentModal)
            return;
        if (closing.current)
            return;
        closing.current = true;
        try {
            const shouldStayOpen = await currentModal?.onRequestClose?.apply(modalContext!);

            if (!shouldStayOpen) {
                forceCloseModal();
            }
        } catch (e) {
            console.error("Failed to close modal", e);
        } finally {
            closing.current = false;
        }
    }, [currentModal, forceCloseModal, modalContext]);

    const kb = useKeybinds(exitModalKeybinds);

    // ensure the root has focus at first so we can listen for escape
    useEffect(() => {
        ref.current?.focus();
    }, []);

    return (
        <LayerPortal>
            {
                transitions((style, context) => (context
                    ? (
                        <animated.div
                            ref={(e) => {
                                updateRef(ref, e);
                                return kb(e);
                            }}
                            style={style}
                            className={cn("absolute top-0 left-0 h-full w-full bg-black/70")}
                            tabIndex={-1}
                            onClick={closeModal}
                        >
                            <ModalContext value={context}>
                                <div onClick={stopParentPropagation}>
                                    <Render context={context} />
                                </div>
                            </ModalContext>
                        </animated.div>
                    )
                    : null))
            }
        </LayerPortal>
    );
}
