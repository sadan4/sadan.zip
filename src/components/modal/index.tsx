import { useComposedRefs } from "@/hooks/composedRefs";
import cn from "@/utils/cn";
import { visibleIf } from "@/utils/react";

import { ModalContext } from "./context";
import * as styles from "./styles.module.scss";
import { Layer } from "../Layer";

import { Activity, type ComponentPropsWithoutRef, type Ref, Suspense, useEffect, useImperativeHandle, useMemo, useRef, useState } from "react";

export {
    ModalContext,
} from "./context";


export interface ModalProps extends ComponentPropsWithoutRef<"dialog"> {
    innerRef?: Ref<HTMLDialogElement>;
    ref: Ref<ModalContext>;
    open?: boolean;
}

export namespace Modal {
}

export function Modal({ children, ref: _ref, className, innerRef, open: _open, ...props }: ModalProps) {
    const [open, setOpen] = useState<boolean>(false);
    const ref = useRef<HTMLDialogElement>(null);
    const dialogRef = useComposedRefs(ref, innerRef);

    const api = useMemo<ModalContext>(() => ({
        open() {
            setOpen(true);
            ref.current?.showModal();
        },
        close() {
            setOpen(false);
            ref.current?.close();
        },
        requestClose() {
            if (ref.current?.dispatchEvent(new Event("cancel", { cancelable: true }))) {
                setOpen(false);
                ref.current.close();
            }
        },
        status: open,
    }), [open]);

    useImperativeHandle(_ref, () => api, [api]);

    useEffect(() => {
        if (typeof _open === "boolean") {
            if (_open) {
                api.open();
            } else {
                api.close();
            }
        }
    }, [_open, api]);

    return (
        <ModalContext value={api}>
            <Suspense>
                <dialog
                    ref={dialogRef}
                    className={cn(styles.modal, className)}
                    {...props}
                >
                    <div
                        className={styles.centerWrapper}
                        onClick={() => {
                            api.requestClose();
                        }}
                    >
                        <div
                            className={styles.content}
                            onClick={(e) => {
                                e.stopPropagation();
                            }}
                        >
                            <Layer>
                                <Activity
                                    name="Modal.children"
                                    mode={visibleIf(open)}
                                >
                                    <div className="fixed inset-fill">
                                        {children}
                                    </div>
                                </Activity>
                            </Layer>
                        </div>
                    </div>
                </dialog>
            </Suspense>
        </ModalContext>
    );
}
