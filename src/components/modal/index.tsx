import { type Keybind, KeybindKeys } from "@/hooks/keybind";
import type { Thenable } from "@/utils/types";

import { SYM_INTERNAL_KEY, useModalStackStore } from "./internal/modalStackStore";
import type { ModalKey } from "../modals/ModalKey";
export {
    ModalKey,
} from "../modals/ModalKey";

import type { JSX } from "react";


export interface Modal {
    [SYM_INTERNAL_KEY]: symbol;
    key?: ModalKey;
    Render(this: IModalContext): JSX.Element;
    /**
     * called when the modal is rendered
     */
    onModalOpen?(this: IModalContext): void;
    /**
     * called when the modal is closed
     */
    onModalClose?(this: IModalContext): void;
    /**
     * called when the modal is requested to close by clicking outside of it
     *
     * return true to stay open
     */
    onRequestClose?(this: IModalContext): Thenable<boolean | void>;
}

export interface IModalContext extends Modal {
    close(): void;
}

export function openModal(modal: Omit<Modal, typeof SYM_INTERNAL_KEY>): void;
export function openModal(_modal: any): void {
    const modal: Modal = { ..._modal };

    modal[SYM_INTERNAL_KEY] = Symbol(`modal.internal.instance.key${modal.key ? `?${modal.key}` : ""}`);

    Object.freeze(modal);

    useModalStackStore
        .getState()
        .pushModal(modal);
}

export const exitModalKeybinds: Keybind[] = [
    {
        key: KeybindKeys.ESCAPE,
        handle() {
            useModalStackStore.getState()
                .popAllModals();
        },
        timing: "down",
    },
];

