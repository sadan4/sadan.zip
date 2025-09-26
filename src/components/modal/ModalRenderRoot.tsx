import { useKeybinds } from "@/hooks/keybind";
import { z } from "@/styles";
import cn from "@/utils/cn";
import { updateRef } from "@/utils/ref";
import { animated, useTransition } from "@react-spring/web";

import { SYM_INTERNAL_KEY, useModalStackStore } from "./internal/modalStackStore";
import { ModalContext } from "./context";
import { exitModalKeybinds } from ".";

import { type BaseSyntheticEvent, useCallback, useEffect, useRef } from "react";


function stopParentPropagation(ev: BaseSyntheticEvent) {
    ev.stopPropagation();
}

export default function ModalRenderRoot() {
    const currentModal = useModalStackStore((state) => state.modals.at(-1));

    const transitions = useTransition(currentModal, {
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
            currentModal.onModalClose?.apply(currentModal);
            useModalStackStore.getState()
                ._popModalByInternalKey(currentModal[SYM_INTERNAL_KEY]);
        } catch (e) {
            console.log("Failed to close modal", e);
            throw e;
        }
    }, [currentModal]);

    const closeModal = useCallback(async () => {
        if (!currentModal)
            return;
        if (closing.current)
            return;
        closing.current = true;
        try {
            const shouldStayOpen = await currentModal?.onRequestClose?.apply(currentModal);

            if (!shouldStayOpen) {
                forceCloseModal();
            }
        } catch (e) {
            console.error("Failed to close modal", e);
        } finally {
            closing.current = false;
        }
    }, [currentModal, forceCloseModal]);

    const kb = useKeybinds(exitModalKeybinds);

    // ensure the root has focus at first so we can listen for escape
    useEffect(() => {
        ref.current?.focus();
    }, []);

    return transitions((style, item) => (item
        ? (
            <animated.div
                ref={(e) => {
                    updateRef(ref, e);
                    return kb(e);
                }}
                style={style}
                className={cn("absolute top-0 left-0 h-full w-full bg-black/70", z.modal)}
                tabIndex={-1}
                onClick={closeModal}
            >
                <ModalContext value={item}>
                    <div onClick={stopParentPropagation}>
                        <item.render />
                    </div>
                </ModalContext>
            </animated.div>
        )
        : null));
}
