export default function error(message?: string): never {
    throw new Error(message);
}
