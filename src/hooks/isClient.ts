import { useEffect, useState } from "react";

export function useIsClient() {
    const [client, setClient] = useState(false);

    useEffect(() => {
        // it will only ever run once
        // oxlint-disable-next-line react/react-compiler
        setClient(true);
    }, []);

    return client;
}
