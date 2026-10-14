import { describe, expect, test } from 'bun:test';

import { orderSpecs } from './run.cjs';

describe('orderSpecs', () => {
  test('orderSpecs with numbered specs returns them by number', () => {
    expect(orderSpecs(['020-tabs.cjs', '010-editor.cjs', '100-agent.cjs'])).toEqual([
      '010-editor.cjs',
      '020-tabs.cjs',
      '100-agent.cjs',
    ]);
  });

  test('orderSpecs with an unnumbered spec throws its name', () => {
    expect(() => orderSpecs(['010-editor.cjs', 'tabs.cjs'])).toThrow('tabs.cjs');
  });

  test('orderSpecs with a file of another kind throws its name', () => {
    expect(() => orderSpecs(['010-editor.cjs', '020-tabs.mjs'])).toThrow('020-tabs.mjs');
  });

  test('orderSpecs with a malformed name throws the expected pattern', () => {
    expect(() => orderSpecs(['010_Editor.cjs'])).toThrow('NNN-nom.cjs');
  });

  test('orderSpecs with a duplicate number throws both names', () => {
    expect(() => orderSpecs(['010-editor.cjs', '010-tabs.cjs'])).toThrow(
      '010-editor.cjs, 010-tabs.cjs',
    );
  });

  test('orderSpecs with no spec throws', () => {
    expect(() => orderSpecs([])).toThrow('aucune spec');
  });
});
