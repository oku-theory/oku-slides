// Waits for the host's answer to a UI-state save (`cmd: 'ui'`): the save leaves after the render and can
// lag on a loaded machine (t-668). Start it BEFORE the action that triggers the save.

/** Resolves with the response to the first `ui` save whose request satisfies `ok(req)`. */
const uiSaved = (p, ok) =>
  p.waitForResponse((res) => {
    const q = res.request();
    if (q.method() !== 'POST') return false;
    try {
      const { req } = JSON.parse(q.postData() ?? '');
      return req?.cmd === 'ui' && ok(req);
    } catch {
      return false;
    }
  });

module.exports = { uiSaved };
