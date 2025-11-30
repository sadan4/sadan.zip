import { useLoaderData } from "@/main";

import { SnowCanvas } from "./effects/SnowCanvas";

export interface BoilerplateProps {
}

export function Boilerplate() {
    const loaderData = useLoaderData();
    const showSnow = !loaderData?.config?.solidBg;

    return showSnow ? <SnowCanvas /> : null;
}
