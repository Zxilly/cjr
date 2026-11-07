import { afterEach, expect, it } from 'vitest';
import { detectLanguage } from './browser';

afterEach(() => {
  localStorage.clear();
  window.history.replaceState({}, '', '/');
});

it('prefers the explicit locale route over stored and query preferences', async () => {
  localStorage.setItem('cjr.language', 'en');
  window.history.replaceState({}, '', '/cjr/zh-CN/?lang=en');
  expect(await detectLanguage('/cjr/')).toBe('zh-CN');
});

it('accepts legacy language links and falls back for other locales', async () => {
  window.history.replaceState({}, '', '/?lang=zh-TW');
  expect(await detectLanguage('/')).toBe('zh-CN');
  localStorage.clear();
  window.history.replaceState({}, '', '/?lang=fr');
  expect(await detectLanguage('/')).toBe('en');
});
