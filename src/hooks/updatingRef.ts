import { useState } from "react";

export function useUpdatingRef<T extends {}>(value: T | null = null) {
    return useState<T | null>(value);
}
