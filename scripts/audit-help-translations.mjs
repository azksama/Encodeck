import fs from 'node:fs';
import ts from 'typescript';

const root = new URL('../', import.meta.url);
const source = fs.readFileSync(new URL('src/lib/fieldHelp.ts', root), 'utf8');
const code = ts.transpileModule(source, {compilerOptions: {module: ts.ModuleKind.ESNext}}).outputText;
const {default: help} = await import('data:text/javascript;base64,' + Buffer.from(code).toString('base64'));
const texts = [...new Set(Object.values(help).flatMap(entry => [entry.text, ...Object.values(entry.values ?? {})]))];
const directory = new URL('src/locales/', root);
let incomplete = false;
for (const file of fs.readdirSync(directory).filter(file => /^[a-z]{2,3}(-[A-Z]{2})?\.json$/.test(file))) {
  const catalog = JSON.parse(fs.readFileSync(new URL(file, directory), 'utf8'));
  const missing = texts.filter(text => !catalog[text]?.trim());
  console.log(`${file}: ${texts.length - missing.length}/${texts.length} help texts`);
  if (missing.length) incomplete = true;
}
if (incomplete) process.exitCode = 1;
