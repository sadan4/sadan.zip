import type { PropsWithChildren } from "react";

export interface BaseBorderHoldProps extends PropsWithChildren {
    onHold?: () => void;
}

declare module "react" {
    interface CSSProperties {
        "--border-hold-progress"?: number;
    }
}
