import { useIsClient } from "@/hooks/isClient";

import { PropsWithChildren } from "react";

export function ClientRender({ children }: PropsWithChildren) {
    const isClient = useIsClient();

    return isClient ? <>{children}</> : null;
}
