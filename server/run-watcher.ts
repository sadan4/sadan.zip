import { Worker } from "node:worker_threads";

new Worker(new URL("./watcher.ts", import.meta.url));
