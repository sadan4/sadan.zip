import { useLoaderData } from "@/main";

import { useEffect } from "react";

export interface BoilerplateProps {
}

export function Boilerplate() {
    const loaderData = useLoaderData();
    const gifBg = !loaderData?.config?.solidBg;

    useEffect(() => {
        if (gifBg) {
            document.body.classList.add("snow");
            return () => {
                document.body.classList.remove("snow");
            };
        }
    }, [gifBg]);
    return null;
}
