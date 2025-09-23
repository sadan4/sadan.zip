import styles from "./styles.module.css";

export const colors = {
    primary: styles.primary,
    secondary: styles.secondary,
    accent: styles.accent,
    neutral: styles.neutral,
    "neutral-content": styles.neutralContent,
    info: styles.info,
    "info-700": styles.info700,
    success: styles.success,
    warning: styles.warning,
    error: styles.error,
} as const;

export const colorTypes = {
    outline: styles.outline,
    filled: styles.filled,
    text: styles.text,
} as const;

