import { SnowCanvas } from "./effects/SnowCanvas/index.tsrx";

export interface BoilerplateProps {
    /**
     * @default false
     */
    solidBg?: boolean;
}

export component Boilerplate({ solidBg = false }: BoilerplateProps) {
    if (!solidBg) {
        <SnowCanvas />
    }
}
