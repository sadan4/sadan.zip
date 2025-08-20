export function copy(text: string): Promise<void> {
    return navigator.clipboard.writeText(text);
}
