import styles from "./styles.module.scss";

import { useEffect } from "react";

export interface BoilerplateProps {
    solidBg?: boolean;
}

export function Boilerplate({ solidBg = false }: BoilerplateProps) {
    useEffect(() => {
        if (!solidBg) {
            document.body.classList.add(styles.snowGif);
            return () => {
                document.body.classList.remove(styles.snowGif);
            };
        }
    }, [solidBg]);
    return null;
}
