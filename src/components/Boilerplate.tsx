import { useEffect } from "react";

export interface BoilerplateProps {
    solidBg?: boolean;
}

export function Boilerplate({ solidBg = false }: BoilerplateProps) {
    useEffect(() => {
        if (!solidBg) {
            document.body.classList.add("snow");
            return () => {
                document.body.classList.remove("snow");
            };
        }
    }, [solidBg]);
    return null;
}
