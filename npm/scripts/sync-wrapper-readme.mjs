#!/usr/bin/env node

import { syncWrapperReadme } from "./readme.mjs";

const [, , outputPath] = process.argv;

console.log(syncWrapperReadme(outputPath));
