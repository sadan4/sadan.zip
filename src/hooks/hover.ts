import _useHover, { type UseHoverOptions } from "@react-hook/hover";

import type { RefObject } from "react";

export const useHover
    = <T extends HTMLElement | null>
    (target: RefObject<T> | T, options?: UseHoverOptions) => _useHover((target as any)?.current || target, options);

