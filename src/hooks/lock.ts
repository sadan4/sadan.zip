import { Lock } from "@/utils/Lock";

import { useState } from "react";

export function useLock(initialState = false): Lock {
    const [lock] = useState<Lock>(() => new Lock(initialState));

    return lock;
}
