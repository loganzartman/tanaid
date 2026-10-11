2. Focus stealing. You're right; with the window never closing, the effect doesn't re-run. Your new close-on-result change could bring it back, though. While you type a Tk script with Auto-run on, a run that fails on a syntax error now closes the window. The next good run reopens it, and the effect moves focus into it. I can't confirm this headless, because the Tk window needs a GPU, so it's worth trying by hand.

3. Close button outline (done). It now gets a dotted outline 2px inside its border when it has keyboard focus. I compared offsets in light and dark mode, and -2px is the one that stays clear of the X. Mouse clicks still show no outline, as on Windows. The change is in packages/playground/98.css.

4. Click to focus. Yes, and you don't need extra state, because the browser already knows: if (!windowRef.matches(":focus-within")) windowRef.focus();. That stops the window grabbing focus back when you click inside it while it's already active.

A shorter route: browsers already behave like Windows here. I checked in Chromium that clicking a non-focusable part of a tabindex=0 window focuses the window, and clicking a control focuses the control. The only exception is the draggable title bar, because its handler cancels pointerdown. So you could delete the window-level onPointerDown and call windowRef.focus() in handleTitlebarDown.

6. Binding errors. Opened as https://github.com/loganzartman/tanaid/issues/39. While writing it I found that my earlier advice to parse scripts when bind runs was wrong for Tk: % substitutions (a TODO in handle_event) rewrite the script text when the event fires, and Tk reports binding errors at that point. The issue suggests parsing inside TkEvent::dispatch instead, so parse errors take the same path as runtime errors.

7. Stuck keys. No DOM API tells you which keys are held. getModifierState only covers modifiers, so you do have to track keys yourself, but it's small:

- Keep a Map keyed by event.code. event.key can change between press and release: hold a, press Shift, release a, and the keyup says A.
- Delete the entry on keyup.
- On focusout, if relatedTarget is outside the window, send a release for each entry and clear the map. The focused element also gets focusout when the tab or browser loses focus, so this covers alt-tab.
- Closing the window doesn't need handling, since it stops the worker anyway.

8. Meta/Command (done). Meta, M and Command now give the Meta modifier, and I dropped Extended, which in Tk marks extended keys rather than a modifier. <Extended-…> bindings no longer parse. I added a test for the aliases; all 14 tanaid-tk tests pass and the formatting checks are clean.

tag.to_string(): a HashMap<String, _> can be looked up with a &str, and in that loop tag is a &&str. So this avoids the allocation:

let Some(bindings) = self.bindings.get(*tag) else {
