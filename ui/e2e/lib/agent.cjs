// Sending a prompt to the agent of a tab, shared by the specs that drive the fake agent.
const { post } = require('./http.cjs');

/** Sends `text` as the user's prompt in `tab` (the fake agent answers on its own). */
const prompt = (p, text, tab = 't-1') => post(p, '/api/agent', { cmd: 'prompt', tab, text, ui: {} });

module.exports = { prompt };
