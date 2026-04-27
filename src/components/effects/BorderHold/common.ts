import type { SpringConfig } from "@react-spring/web";

import { type PropsWithChildren } from "react";

export interface BaseBorderHoldProps extends PropsWithChildren {
    onHold?: () => void;
}

declare module "react" {
    interface CSSProperties {
        "--border-hold-progress"?: number;
    }
}

export interface UseBorderHoldAnimProps {
    onHold?(): void;
    held: boolean;
}

export function borderHoldAnimConfig(held: boolean) {
    return (k: "opacity" | "progress"): SpringConfig => {
        switch (k) {
            case "opacity":
                return {};
            case "progress":
                return {
                    mass: 5,
                    friction: held ? 75 : 50,
                };
        }
    };
}

