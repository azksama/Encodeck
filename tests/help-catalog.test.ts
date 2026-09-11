import {describe, expect, it} from 'vitest';
import help from '../src/lib/fieldHelp';
import source from '../src/locales/en.json';

describe('Help translation extraction', () => {
  for (const [id, entry] of Object.entries(help)) {
    it(`includes the description and every option note for ${id}`, () => {
      for (const text of [entry.text, ...Object.values(entry.values ?? {})]) {
        expect(source, `Unextracted help text: ${text}`).toHaveProperty(text);
      }
    });
  }
});
