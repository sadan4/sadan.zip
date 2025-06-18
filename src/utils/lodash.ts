import { clamp, mapValues, round } from "lodash-es";

const __chainableFunctions = {
    clamp,
    mapValues,
    round,
};

if (window[Symbol()]) {
    __chainableFunctions;
}
