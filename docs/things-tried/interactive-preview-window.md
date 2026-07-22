# Things tried: interactive preview window

An earlier version of the homepage hero was fully interactive: a "browser
window" (an iframe showing `assets/sample.html`) with **working minimize,
maximize/restore, and close buttons**, a macOS-style dock that reappeared
when the window was hidden, and an animated snowy desktop scene behind it —
falling snow, birds in flight, a mammoth trunk swaying at the edge of the
frame, and a live clock in the top bar.

It was retired when the homepage moved to Markdown-only source
(`docs/index.md`): mdo sanitizes raw HTML by default and emits no JavaScript
hooks, so a widget that needs `<div class>` markup, an iframe, and a script
cannot be expressed in the page source anymore. The current hero is the
CSS-only [faux browser window](../faux-browser-window.html), which trades
interactivity for a pure-Markdown source.

The pieces are preserved here in working order for possible revival — for
example on a hand-written demo page, which is allowed to use raw HTML the
same way the metrics pages do.

## The pieces

- **CSS** — [`interactive-preview-window.css`](https://github.com/maphew/mdo/blob/main/docs/things-tried/interactive-preview-window.css)
  in this directory: the window states (`data-preview-state` =
  `maximized` / `windowed` / `minimized` / `closed`), toolbar and window
  controls, dock, and the animated desktop scene, with the CSS variables
  inlined so the file stands alone.
- **Behavior** — the state machine below (originally
  `docs/assets/site.js`). Each control sets `data-preview-state` on the
  shell; CSS transitions do the rest. It also manages `inert`,
  `aria-hidden`, and tab order so hidden surfaces leave the accessibility
  tree, and keeps the fake desktop clock current.
- **Markup** — the shell structure below (originally in the hand-written
  `docs/index.html`).
- **Last working page** — `git show 2ba95ed:docs/index.html` and
  `git show 2ba95ed:docs/assets/site.js` ("Animate preview desktop easter
  egg").

## Markup

```html
<div class="preview-shell" data-preview-shell data-preview-state="maximized"
     aria-label="Rendered Markdown example desktop">
  <div class="desktop-scene" aria-hidden="true">
    <div class="desktop-topbar">
      <span data-preview-clock>Mon Jan 27&nbsp;&nbsp;1:52 PM</span>
    </div>
    <div class="snowfall"><!-- 16 empty <span>s --></div>
    <div class="bird-flight"><!-- 5 empty <span>s --></div>
    <div class="perched-birds"><!-- 3 empty <span>s --></div>
    <div class="herd-trail"></div>
    <div class="mammoth-motion">
      <span class="mammoth-trunk"></span>
      <span class="mammoth-foot"></span>
    </div>
  </div>
  <div class="preview-window">
    <div class="preview-toolbar">
      <span class="preview-title">sample.md rendered by mdo</span>
      <span class="preview-window-actions">
        <button class="window-control minimize" type="button"
                data-preview-action="minimize" aria-label="Minimize preview"></button>
        <button class="window-control maximize" type="button"
                data-preview-action="maximize" aria-label="Maximize preview"></button>
        <button class="window-control close" type="button"
                data-preview-action="close" aria-label="Close preview"></button>
      </span>
    </div>
    <iframe class="preview-frame" src="assets/sample.html"
            title="Example Markdown rendered as HTML"></iframe>
  </div>
  <div class="preview-dock" aria-label="Preview dock">
    <button class="dock-launcher" type="button" data-preview-action="restore"
            aria-label="Open rendered Markdown preview" tabindex="-1" disabled>
      <span class="firefox-mark" aria-hidden="true"></span>
    </button>
  </div>
</div>
```

## Behavior

```js
(() => {
  const shell = document.querySelector("[data-preview-shell]");

  if (!shell) {
    return;
  }

  const previewWindow = shell.querySelector(".preview-window");
  const dock = shell.querySelector(".preview-dock");
  const dockLauncher = shell.querySelector("[data-preview-action='restore']");
  const maximizeButton = shell.querySelector("[data-preview-action='maximize']");
  const clock = shell.querySelector("[data-preview-clock]");
  const actions = shell.querySelectorAll("[data-preview-action]");

  function updateClock() {
    if (!clock) {
      return;
    }

    const now = new Date();
    const dateParts = new Intl.DateTimeFormat(undefined, {
      weekday: "short",
      month: "short",
      day: "numeric",
    }).formatToParts(now);
    const time = new Intl.DateTimeFormat(undefined, {
      hour: "numeric",
      minute: "2-digit",
    }).format(now);
    const getPart = (type) => dateParts.find((part) => part.type === type)?.value;

    clock.textContent = `${getPart("weekday")} ${getPart("month")} ${getPart("day")}  ${time}`;
  }

  function setState(state) {
    shell.dataset.previewState = state;

    const desktopVisible = state !== "maximized";
    const windowHidden = state === "minimized" || state === "closed";

    previewWindow.toggleAttribute("inert", windowHidden);
    previewWindow.setAttribute("aria-hidden", windowHidden ? "true" : "false");
    dock.setAttribute("aria-hidden", desktopVisible ? "false" : "true");
    dockLauncher.disabled = !desktopVisible;
    dockLauncher.tabIndex = desktopVisible ? 0 : -1;
    maximizeButton.setAttribute(
      "aria-label",
      state === "windowed" ? "Maximize preview" : "Restore preview window",
    );
  }

  actions.forEach((action) => {
    action.addEventListener("click", () => {
      const nextAction = action.dataset.previewAction;
      const currentState = shell.dataset.previewState || "maximized";

      if (nextAction === "minimize") {
        setState("minimized");
      } else if (nextAction === "close") {
        setState("closed");
      } else if (nextAction === "restore") {
        setState("maximized");
      } else if (nextAction === "maximize") {
        setState(currentState === "windowed" ? "maximized" : "windowed");
      }
    });
  });

  setState(shell.dataset.previewState || "maximized");
  updateClock();
  setInterval(updateClock, 15000);
})();
```

## Reviving it

Drop the markup into a hand-written HTML page, link the CSS file, and include
the script. The iframe path and the background image URL
(`../assets/mammoth-bluefinhero-1024x695.jpg`) assume the page sits in
`docs/`; adjust for another location. The states are driven entirely by the
`data-preview-state` attribute, so the widget is easy to test by setting the
attribute by hand before wiring up the script.
