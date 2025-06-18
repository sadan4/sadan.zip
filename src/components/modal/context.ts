import type { Modal } from ".";

import { createContext } from "react";

export const ModalContext = createContext<Readonly<Modal> | null>(null);
