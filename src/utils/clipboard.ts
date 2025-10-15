export function copy(text: string): Promise<void> {
    return navigator.clipboard.writeText(text);
}
export function paste(): Promise<string> {
    return navigator.clipboard.readText();
}
