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
