import '@fontsource/patua-one/latin-400.css';
import '@fontsource-variable/jetbrains-mono';
import '@fontsource-variable/noto-sans-sc';
import './style.css';

const english = {
  skip: 'Skip to content', navigation: 'Site navigation', language: 'Language',
  headline: 'Run Cangjie programs.',
  intro: 'Start your Cangjie program with one command. Skip the search for runtimes and the manual setup.',
  quickstart: 'Get started', steps: 'Getting started steps',
  install: 'Install cjr', run: 'Run a program', manage: 'Inspect & clean',
  installTitle: 'Install from source', requirements: 'Requires Rust 1.89+, a C compiler and CMake.',
  copyInstall: 'Copy installation command', installDocs: 'Installation guide →',
  runTitle: 'Start with a program path', runHint: 'Arguments after the path go directly to your application.',
  copyRun: 'Copy launch command',
  offline: 'Once cached, run offline with cjr --offline ./app. On Windows, use a path such as .\\app.exe.',
  manageTitle: 'See versions and local content', inspectHint: 'Inspect version and dependencies',
  listHint: 'List local content', cleanHint: 'Preview cache cleanup', commandsDocs: 'Command reference →',
  docs: 'Documentation', userGuide: 'User guide', userGuideHint: 'Installation, commands, cache and troubleshooting',
  devGuide: 'Developer guide', devGuideHint: 'Build from source and contribute',
  scope: 'Supported platforms and limitations →',
  issues: 'Report an issue', cangjie: 'Cangjie',
};
const language = new URLSearchParams(location.search).get('lang') === 'en' ? 'en' : 'zh-CN';
if (language === 'en') {
  document.documentElement.lang = 'en';
  document.title = 'cjr — Run Cangjie programs';
  document.querySelector('meta[name="description"]').content = english.intro;
  for (const element of document.querySelectorAll('[data-text]')) {
    element.textContent = english[element.dataset.text];
  }
  for (const element of document.querySelectorAll('[data-label]')) {
    element.setAttribute('aria-label', english[element.dataset.label]);
  }
}
for (const link of document.querySelectorAll('[data-language]')) {
  if (link.dataset.language === language) link.setAttribute('aria-current', 'true');
  else link.removeAttribute('aria-current');
}
for (const link of document.querySelectorAll('[data-book]')) {
  link.href = `${import.meta.env.BASE_URL}book/${link.dataset.book}/${language}/${link.dataset.page}`;
}

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
      status.textContent = language === 'en' ? 'Command copied' : '命令已复制';
    } catch {
      status.textContent = language === 'en'
        ? 'Copy unavailable. Select the command and copy it manually.'
        : '无法访问剪贴板，请选中命令后手动复制。';
    }
    statusTimer = setTimeout(() => { status.textContent = ''; }, 5000);
  });
}
