import { namedContext } from "@/utils/devtools";

import type { Modal } from ".";


export const ModalContext = namedContext<Readonly<Modal> | null>(null, "ModalContext");
