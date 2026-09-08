#!/usr/bin/env node
/**
 * Copies the canonical OpenAPI spec into the portal public tree.
 *
 * Source of truth: Docs/openapi/altius-ftm-v3.openapi.yaml
 * Portal copy:     apps/developer-portal/public/openapi/altius-ftm-v3.openapi.yaml
 */
import { copyFileSync, mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const portalRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = path.resolve(portalRoot, "../..");
const source = path.join(repoRoot, "Docs/openapi/altius-ftm-v3.openapi.yaml");
const destDir = path.join(portalRoot, "public/openapi");
const dest = path.join(destDir, "altius-ftm-v3.openapi.yaml");

mkdirSync(destDir, { recursive: true });
copyFileSync(source, dest);
console.log(`Synced OpenAPI → ${path.relative(repoRoot, dest)}`);
