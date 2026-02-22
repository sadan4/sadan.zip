import mappings from "./key-mappings.json";

export function tryMapIntlKey(hashedKey: string): string | null {
    if (hashedKey in mappings) {
        return mappings[hashedKey as keyof typeof mappings];
    }
    return null;
}
