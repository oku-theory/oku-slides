// Les feuilles de style de l'interface, lues depuis ui/src pour les specs qui posent un HTML à elles.
const fs = require('node:fs');
const path = require('node:path');

/** Les feuilles de l'app, dans l'ordre où main.tsx les charge : styles.css puis sa suite. */
const APP_SHEETS = ['styles.css', 'import.css', 'pack.css'];

/** Le texte des feuilles `files` (chemins relatifs à ui/src), à la suite. */
const sheets = (...files) => files.map((f) => fs.readFileSync(path.join(__dirname, '..', '..', 'src', f), 'utf8')).join('\n');

module.exports = { APP_SHEETS, sheets };
