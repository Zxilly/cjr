import '@fontsource/patua-one/latin-400.css';
import '@fontsource-variable/jetbrains-mono';
import '@fontsource-variable/noto-sans-sc';
import './style.css';

import i18n from './i18n.js';

const language = i18n.resolvedLanguage;
const t = key => i18n.t(key, { version: __CJR_VERSION__ });
document.documentElement.lang = language;
document.title = t('title');
document.querySelector('meta[name="description"]').content = t('description');
for (const element of document.querySelectorAll('[data-text]')) {
  element.textContent = t(element.dataset.text);
}
for (const element of document.querySelectorAll('[data-label]')) {
  element.setAttribute('aria-label', t(element.dataset.label));
}
for (const link of document.querySelectorAll('[data-language]')) {
  link.href = `${import.meta.env.BASE_URL}${link.dataset.language}/`;
  if (link.dataset.language === language) link.setAttribute('aria-current', 'true');
  else link.removeAttribute('aria-current');
}
for (const link of document.querySelectorAll('[data-book]')) {
  link.href = `${import.meta.env.BASE_URL}book/${link.dataset.book}/${language}/${link.dataset.page}`;
}
for (const link of document.querySelectorAll('[data-asset]')) {
  link.href = `https://github.com/Zxilly/cjr/releases/download/v${__CJR_VERSION__}/cjr-v${__CJR_VERSION__}-${link.dataset.asset}`;
}
document.querySelector('[data-release]').href = `https://github.com/Zxilly/cjr/releases/tag/v${__CJR_VERSION__}`;
document.getElementById('install-command').textContent = `cargo install --git https://github.com/Zxilly/cjr --tag v${__CJR_VERSION__} --locked`;

const tabs = [...document.querySelectorAll('[role="tab"]')];
function selectTab(selected) {
  for (const tab of tabs) {
    const active = tab === selected;
    tab.setAttribute('aria-selected', String(active));
    tab.tabIndex = active ? 0 : -1;
    document.getElementById(tab.getAttribute('aria-controls')).hidden = !active;
  }
}
for (const tab of tabs) {
  tab.addEventListener('click', () => selectTab(tab));
  tab.addEventListener('keydown', event => {
    let index = tabs.indexOf(tab);
    if (event.key === 'ArrowRight') index = (index + 1) % tabs.length;
    else if (event.key === 'ArrowLeft') index = (index + tabs.length - 1) % tabs.length;
    else if (event.key === 'Home') index = 0;
    else if (event.key === 'End') index = tabs.length - 1;
    else return;
    event.preventDefault();
    selectTab(tabs[index]);
    tabs[index].focus();
  });
}
const status = document.getElementById('copy-status');
let statusTimer;
for (const button of document.querySelectorAll('[data-copy]')) {
  button.addEventListener('click', async () => {
    clearTimeout(statusTimer);
    try {
      await navigator.clipboard.writeText(document.getElementById(button.dataset.copy).textContent);
      status.textContent = t('copied');
    } catch {
      status.textContent = t('copyFailed');
    }
    statusTimer = setTimeout(() => { status.textContent = ''; }, 5000);
  });
}
