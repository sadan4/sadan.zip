import * as styles from "./styles.module.scss";

export const colors = {
    primary: styles.primary,
    secodnary: styles.secondary,
    warning: styles.warning,
    error: styles.error,
} as const;

export const colorTypes = {
    outline: styles.outline,
    filled: styles.filled,
    text: styles.text,
};
