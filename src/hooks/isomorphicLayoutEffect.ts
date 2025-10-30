import { useEffect, useLayoutEffect } from "react";

export const useIsomorphicLayoutEffect = typeof process === "undefined" ? useLayoutEffect : useEffect;
