// Identifiants aléatoires courts : relier un envoi à sa réponse, reconnaître une vue. Jamais un secret.

/** Un identifiant aléatoire court, préfixé ; jamais un secret. */
export function randomId(prefix: string): string {
  return `${prefix}-${Math.random().toString(36).slice(2, 12)}`;
}
