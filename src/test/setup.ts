// src/test/setup.ts
// This file will run before each test file

// Import necessary for vitest-browser-react
import "vitest-browser-react";

// Add any global setup code here
export default function setup() {
    // You can return a cleanup function if needed
    return () => {
        // Cleanup code here
    };
}
