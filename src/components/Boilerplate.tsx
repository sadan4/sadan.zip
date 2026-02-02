import { SnowCanvas } from "./effects/SnowCanvas";

export interface BoilerplateProps {
    /**
     * @default false
     */
    solidBg?: boolean;
}

export function Boilerplate({ solidBg = false }: BoilerplateProps) {
    return !solidBg ? <SnowCanvas /> : null;
}
