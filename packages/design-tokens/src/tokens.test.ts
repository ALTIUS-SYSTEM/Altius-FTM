import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { tokens } from './index';

test('JSON token colors, spacing, radii and typography match Tailwind source', () => {
  const css = readFileSync(new URL('./tailwind.css', import.meta.url), 'utf8');
  for (const [group, prefix] of [['colors', 'color'], ['spacing', 'spacing'], ['rounded', 'radius']] as const) {
    for (const [name, value] of Object.entries(tokens[group])) assert.ok(css.includes(`--${prefix}-${name}: ${value};`));
  }
  for (const [name, value] of Object.entries(tokens.typography)) {
    assert.ok(css.includes(`--text-${name}: ${value.fontSize};`));
    assert.ok(css.includes(`--text-${name}--line-height: ${value.lineHeight};`));
    assert.ok(css.includes(`--text-${name}--font-weight: ${value.fontWeight};`));
  }
  assert.equal(tokens.colors.action, '#0fb4da');
  assert.ok(Object.isFrozen(tokens.colors));
  assert.ok(Object.isFrozen(tokens.typography['body-lg']));
});
