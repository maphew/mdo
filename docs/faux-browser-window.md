# Faux Browser Window How-To

The [mdo homepage](https://maphew.github.io/mdo/) opens with what looks like a
screenshot: a small browser window, complete with title bar and minimize /
maximize / close controls, floating over the hero image. It is not a
screenshot, an iframe, or embedded raw HTML. The source is plain Markdown —
an image followed by a blockquote — and everything else is layered on with
`mdo --css`.

This page explains how the trick works so you can adapt it for your own
pages. For the general `--css` mechanics and cascade order, see
[Custom CSS](custom-css.html).

## Step 1 — Write a recognizable Markdown shape

The homepage source (`docs/index.md`) contains only this:

```markdown
![A mammoth beside a bluefin tuna in a snowy prehistoric scene.](assets/mammoth-bluefinhero-1024x695.jpg)

> sample.md rendered by mdo
>
> # Release Notes Draft
>
> `mdo` turns Markdown into a standalone HTML5 document with embedded styling.
>
> ## Open in Browser
>
> - [x] Convert Markdown once and exit
> ...
```

Two structural choices make the CSS possible:

- The blockquote **immediately follows** the image paragraph, so a CSS
  adjacent-sibling selector (`+`) can find it.
- The blockquote's **first paragraph is a short caption**
  (`sample.md rendered by mdo`), which the CSS will turn into the window's
  title bar.

Anywhere the stylesheet is absent — GitHub's Markdown view, a stock `mdo`
render, another converter — the same source degrades gracefully to an image
followed by an ordinary quotation. The content stays portable and readable.

## Step 2 — Gate the styling to mdo output

mdo appends a small `<footer class="mdo-source-meta">` to generated pages.
The stylesheet uses that as a fingerprint so the fancy rules only apply to
mdo-rendered pages, not to hand-written HTML that happens to share the CSS
file:

```css
body:has(> footer.mdo-source-meta) > main > blockquote {
  /* only mdo-generated pages match */
}
```

## Step 3 — Float the window over the image

The core move is an adjacent-sibling selector anchored on the hero image,
plus a negative top margin that pulls the blockquote up and over it:

```css
body:has(> footer.mdo-source-meta)
  > main
  > p:has(> img[src$="mammoth-bluefinhero-1024x695.jpg"])
  + blockquote {
  position: relative;
  z-index: 1;
  width: min(430px, 52%);
  min-height: 360px;
  max-height: 390px;
  margin: -460px 0 88px clamp(24px, 36%, 430px);
  padding: 0 1.25rem 1.25rem;
  overflow: hidden;
  border: 1px solid rgba(213, 225, 232, 0.92);
  border-radius: 8px;
  background: #fff;
  box-shadow: 0 16px 36px rgba(7, 18, 24, 0.3);
  color: #161d22;
  font-style: normal;
}
```

Matching on `img[src$="..."]` pins the effect to this one image, so other
image-plus-quote sequences on the site are left alone. `overflow: hidden`
with a `max-height` crops the quoted document mid-scroll, which is what sells
the "live page in a window" illusion.

## Step 4 — Turn the caption into a title bar

The blockquote's first paragraph is restyled as the window chrome, and a
`::after` pseudo-element paints the window controls — the minimize, maximize,
and close glyphs are literally the text `−   □   ×`:

```css
/* selector prefix as in step 3, abbreviated here */
... + blockquote > p:first-child {
  display: flex;
  min-height: 48px;
  align-items: center;
  margin: 0 -1.25rem 1.35rem;   /* stretch across the window edge-to-edge */
  border-bottom: 1px solid #d9dde0;
  background: #edf1f3;
  color: #5c666b;
}

... + blockquote > p:first-child::after {
  content: "−   □   ×";
  position: absolute;
  top: 0;
  right: 0;
  width: 8rem;
  height: 100%;
  display: grid;
  place-items: center;
  border-left: 1px solid rgba(139, 154, 162, 0.22);
  letter-spacing: 0.42rem;
  white-space: pre;
}
```

## Step 5 — Finishing touches

- A second `::after` on the blockquote itself draws a slim rounded bar down
  the right edge — a fake scrollbar thumb.
- Headings, tables, and list text inside the window get smaller font sizes so
  a whole document plausibly fits in the frame.
- A `@media (max-width: 760px)` block re-anchors the window (full width,
  smaller negative margin) so the illusion holds on phones.

The complete implementation lives in
[`docs/assets/site.css`](https://github.com/maphew/mdo/blob/main/docs/assets/site.css)
— search for `mammoth` to jump to the faux-window rules.

An earlier, fully interactive version of this window — working minimize /
maximize / close buttons, a dock, and an animated desktop scene — needed raw
HTML and JavaScript, so it could not survive the move to Markdown-only page
source. It is preserved in working order in
[things tried: interactive preview window](things-tried/interactive-preview-window.html).

## Try it yourself

From a checkout of the repository:

```bash
mdo --css docs/assets/site.css --open docs/index.md
```

To build your own version: write the image-then-blockquote shape in plain
Markdown, copy the selectors above, swap in your image's filename, and adjust
the negative margin and width until the window sits where you want it. The
Markdown stays honest; the story is all in the stylesheet.
