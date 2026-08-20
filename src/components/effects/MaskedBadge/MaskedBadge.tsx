import type { BadgePosition } from "./enums";

import type { PropsWithChildren, ReactNode } from "react";

export interface MaskedBadgeProps extends PropsWithChildren {
    renderMask(): ReactNode;
    /**
     * @default BadgePosition.BOTTOM_LEFT
     */
    position?: BadgePosition;
}

export function MaskedBadge({ children: _ }: MaskedBadgeProps) {
}
