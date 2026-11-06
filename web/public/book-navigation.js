// Explicit document URLs select the language; remember it for visits to the homepage.
try { localStorage.setItem('cjr.language', document.documentElement.lang); } catch { /* Storage may be disabled. */ }
