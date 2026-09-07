import test from 'node:test';
import assert from 'node:assert/strict';
import { catalogs, locales, resolveLocale, translate } from './index';

test('seven original catalogs have identical nonempty keys', () => {
  assert.equal(locales.length, 7);
  for (const catalog of Object.values(catalogs)) {
    assert.deepEqual(Object.keys(catalog).sort((a, b) => a.localeCompare(b)), Object.keys(catalogs.en).sort((a, b) => a.localeCompare(b)));
    assert.ok(Object.values(catalog).every(value => value.trim().length > 0));
    assert.equal(catalog['app.name'], 'Altius Field');
  }
});
test('locale normalization and unsupported language fallback', () => {
  assert.equal(resolveLocale('fil_PH'), 'fil-PH');
  assert.equal(resolveLocale('id-ID'), 'id');
  assert.equal(resolveLocale('zh-CN'), 'zh');
  assert.equal(resolveLocale('unknown'), 'en');
  assert.equal(translate('id', 'driver.arrive'), 'Lapor tiba');
});
