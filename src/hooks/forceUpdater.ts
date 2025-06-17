import { type ActionDispatch, useReducer } from "react";

export function useForceUpdater(): [dep: number, update: ActionDispatch<[]>] {
    return useReducer((x) => x + 1, 0);
}
