// @ts-nocheck

import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import prettier from 'prettier';
import openapiTS, { astToString } from 'openapi-typescript';
import { fileURLToPath } from 'node:url';

const sourceDirectory = dirname(fileURLToPath(import.meta.url));
const repositoryRoot = resolve(sourceDirectory, '../../../../..');
const contractPath = resolve(repositoryRoot, 'docs/msc2/api-contract/openapi.json');
const outputPath = resolve(sourceDirectory, 'generated.ts');
const generatedHeader =
  '// Generated from docs/msc2/api-contract/openapi.json. Do not edit by hand.';

async function generatedSource(): Promise<string> {
  const contractText = await readFile(contractPath, 'utf8');
  const contract = JSON.parse(contractText);
  const ast = await openapiTS(contract, {
    additionalProperties: true,
    alphabetize: true,
  });
  const types = await prettier.format(astToString(ast), {
    parser: 'typescript',
    singleQuote: false,
  });
  const contractHash = createHash('sha256').update(contractText).digest('hex');

  return `${generatedHeader}\n// Contract SHA-256: ${contractHash}\n\n${types.trim()}\n`;
}

const expected = await generatedSource();
const checkOnly = process.argv.includes('--check');

if (checkOnly) {
  let actual: string;
  try {
    actual = await readFile(outputPath, 'utf8');
  } catch {
    throw new Error(`generated API types are missing: ${outputPath}`);
  }

  if (actual !== expected) {
    throw new Error(
      'generated API types are stale; run `npm run api:generate` and commit the resulting generated.ts',
    );
  }

  console.log('OK: generated API types match the frozen OpenAPI contract');
} else {
  await writeFile(outputPath, expected);
  console.log(`wrote ${outputPath}`);
}
