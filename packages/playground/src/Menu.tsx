import { createContext, createUniqueId, useContext, type ParentProps } from "solid-js";
import type { JSX } from "@solidjs/web";

/** How long the pointer rests on an item before its submenu opens or closes, as in Windows. */
const SUBMENU_DELAY_MS = 400;

type MenuParent = {
  /** Whether items sit in the menu bar or in a menu. */
  kind: "bar" | "menu";
  /** Opens the menu that `item` opens. */
  open: (item: HTMLButtonElement, focus: Focus) => void;
  /** Opens the menu bar's next or previous menu. */
  moveInBar: (delta: 1 | -1) => void;
  /** Closes every open menu. */
  closeAll: () => void;
};

/** What to focus when a menu opens: the menu itself, its first item, or nothing. */
type Focus = "menu" | "first-item" | "none";

const MenuParentContext = createContext<MenuParent>();

/** Where each open menu's popover returns focus when it closes. */
const returnFocus = new WeakMap<HTMLElement, HTMLElement | null>();

/** A row of menus, like the one under a window's title bar. */
export function MenuBar(props: ParentProps) {
  // oxlint-disable-next-line no-unassigned-vars
  let bar!: HTMLDivElement;
  // the element that had focus before the menu bar opened a menu
  let focusBeforeOpen: HTMLElement | null = null;

  const items = () => [...bar.querySelectorAll<HTMLButtonElement>(':scope > [role="menuitem"]')];
  const openItem = () => items().find((item) => isOpen(popoverOf(item)));

  const open = (item: HTMLButtonElement, focus: Focus) => {
    if (!popoverOf(item)) {
      item.focus();
      return;
    }
    if (!openItem()) {
      focusBeforeOpen = document.activeElement as HTMLElement | null;
    }
    // keyboard users return to the menu bar; others return to what they were doing
    openMenu(item, focus, bar.contains(focusBeforeOpen) ? item : focusBeforeOpen);
  };

  const moveInBar = (delta: 1 | -1) => {
    const all = items();
    const current = openItem() ?? (document.activeElement as HTMLButtonElement);
    const index = all.indexOf(current);
    if (index === -1) {
      return;
    }
    const next = all[(index + delta + all.length) % all.length];
    if (openItem()) {
      open(next, "first-item");
    } else {
      next.focus();
    }
  };

  const parent: MenuParent = {
    kind: "bar",
    open,
    moveInBar,
    closeAll: () => {
      const item = openItem();
      if (item) {
        popoverOf(item)?.hidePopover();
      }
    },
  };

  return (
    <div
      ref={bar}
      role="menubar"
      onPointerMove={(event) => {
        // once a menu is open, pointing at another item opens its menu instead
        const item = childOf(bar, event.target as Element);
        const current = openItem();
        if (current && item !== current && item instanceof HTMLButtonElement && popoverOf(item)) {
          open(item, "menu");
        }
      }}
      onKeyDown={(event) => {
        // keys pressed in a menu bubble up through the menu bar
        if ((event.target as Element).parentElement !== bar) {
          return;
        }
        const item = event.target as HTMLButtonElement;
        switch (event.key) {
          case "ArrowLeft":
            moveInBar(-1);
            break;
          case "ArrowRight":
            moveInBar(1);
            break;
          case "ArrowDown":
            if (!popoverOf(item)) {
              return;
            }
            open(item, "first-item");
            break;
          default:
            return;
        }
        event.preventDefault();
      }}
    >
      <MenuParentContext value={parent}>{props.children}</MenuParentContext>
    </div>
  );
}

export type MenuProps = ParentProps<{
  label: JSX.Element;
}>;

/**
 * An item that opens a menu of its children. In a `MenuBar` it opens below;
 * in another `Menu` it opens a submenu to the side.
 */
export function Menu(props: MenuProps) {
  const parent = useMenuParent();
  const id = createUniqueId();
  const anchorName = `--menu-${id}`;

  // oxlint-disable-next-line no-unassigned-vars
  let item!: HTMLButtonElement;
  // oxlint-disable-next-line no-unassigned-vars
  let popover!: HTMLDivElement;
  // oxlint-disable-next-line no-unassigned-vars
  let menu!: HTMLDivElement;

  // the item or submenu popover under the pointer
  let hovered: Element | null = null;
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;

  const items = () => [...menu.querySelectorAll<HTMLButtonElement>(':scope > [role="menuitem"]')];

  const self: MenuParent = {
    kind: "menu",
    open: (target, focus) => openMenu(target, focus, target),
    moveInBar: parent.moveInBar,
    closeAll: parent.closeAll,
  };

  return (
    <>
      <button
        ref={item}
        role="menuitem"
        aria-haspopup="menu"
        popovertarget={id}
        style={{ "anchor-name": anchorName }}
        onMouseDown={(event) => {
          if (parent.kind === "bar") {
            // the menu takes focus itself, and returns it when it closes
            event.preventDefault();
          }
        }}
        onPointerDown={(event) => {
          if (parent.kind !== "bar" || event.button !== 0) {
            return;
          }
          // menu bar menus open on press, so the pointer can drag down and release on an item
          if (isOpen(popover)) {
            popover.hidePopover();
          } else {
            parent.open(item, "menu");
          }
        }}
        onClick={(event) => {
          // open menus ourselves rather than let `popovertarget` toggle them
          event.preventDefault();
          // menu bar menus opened on press already
          const fromKeyboard = event.detail === 0;
          if (fromKeyboard) {
            parent.open(item, "first-item");
          } else if (parent.kind === "menu" && !isOpen(popover)) {
            parent.open(item, "none");
          }
        }}
        onKeyDown={(event) => {
          if (parent.kind === "menu" && event.key === "ArrowRight") {
            event.preventDefault();
            event.stopPropagation();
            parent.open(item, "first-item");
          }
        }}
      >
        {props.label}
      </button>
      <div
        ref={popover}
        id={id}
        popover
        class="menu-popover"
        style={{ "position-anchor": anchorName }}
        onToggle={(event) => {
          if (event.newState === "open") {
            return;
          }
          clearTimeout(hoverTimer);
          hovered = null;
          // browsers restore focus only on some ways of closing
          const focused = document.activeElement;
          if (!focused || focused === document.body || popover.contains(focused)) {
            returnFocus.get(popover)?.focus({ preventScroll: true });
          }
        }}
      >
        <div
          ref={menu}
          role="menu"
          tabindex={-1}
          // pointer moves bubble up from submenus, which sit inside the menu that opens them.
          // `pointerenter` would also fire when a menu opens under a resting pointer.
          onPointerMove={(event) => {
            const child = childOf(menu, event.target as Element);
            if (!child || child === hovered) {
              return;
            }
            hovered = child;
            clearTimeout(hoverTimer);
            if (child.matches("[popover]")) {
              // the pointer reached an open submenu, so leave it open
              return;
            }
            const target = child as HTMLButtonElement;
            target.focus({ preventScroll: true });
            hoverTimer = setTimeout(() => {
              for (const other of items()) {
                if (other !== target) {
                  popoverOf(other)?.hidePopover();
                }
              }
              if (popoverOf(target) && !isOpen(popoverOf(target))) {
                self.open(target, "none");
              }
            }, SUBMENU_DELAY_MS);
          }}
          onPointerLeave={() => {
            hovered = null;
            const focused = document.activeElement as HTMLButtonElement | null;
            if (focused?.parentElement === menu && !isOpen(popoverOf(focused))) {
              clearTimeout(hoverTimer);
              menu.focus({ preventScroll: true });
            }
          }}
          onKeyDown={(event) => {
            const all = items();
            const index = all.indexOf(document.activeElement as HTMLButtonElement);
            switch (event.key) {
              case "ArrowDown":
                all[(index + 1) % all.length]?.focus();
                break;
              case "ArrowUp":
                all.at(Math.max(index, 0) - 1)?.focus();
                break;
              case "ArrowRight":
                parent.moveInBar(1);
                break;
              case "ArrowLeft":
                if (parent.kind === "menu") {
                  popover.hidePopover();
                } else {
                  parent.moveInBar(-1);
                }
                break;
              default:
                return;
            }
            event.preventDefault();
            event.stopPropagation();
          }}
        >
          <MenuParentContext value={self}>{props.children}</MenuParentContext>
        </div>
      </div>
    </>
  );
}

export type MenuButtonProps = ParentProps<{
  icon?: JSX.Element;
  onClick?: () => void;
}>;

/** An item that closes its menus and runs `onClick`. */
export function MenuButton(props: MenuButtonProps) {
  const parent = useMenuParent();

  const activate = () => {
    parent.closeAll();
    props.onClick?.();
  };

  return (
    <button
      role="menuitem"
      // a press that started on the menu bar can be released here
      onPointerUp={(event) => {
        if (event.button === 0) {
          activate();
        }
      }}
      onClick={(event) => {
        if (event.detail === 0) {
          activate();
        }
      }}
    >
      {props.icon}
      <span>{props.children}</span>
    </button>
  );
}

function useMenuParent() {
  const parent = useContext(MenuParentContext);
  if (!parent) {
    throw new Error("Menu items must be inside a MenuBar");
  }
  return parent;
}

/** The child of `parent` that contains `node`. */
function childOf(parent: Element, node: Element | null) {
  while (node && node.parentElement !== parent) {
    node = node.parentElement;
  }
  return node;
}

function isOpen(popover: Element | null | undefined) {
  return popover?.matches(":popover-open") ?? false;
}

/** The menu popover that `item` opens, if it opens one. */
function popoverOf(item: HTMLButtonElement | null) {
  return (item?.popoverTargetElement ?? null) as HTMLElement | null;
}

function openMenu(item: HTMLButtonElement, focus: Focus, returnTo: HTMLElement | null) {
  const popover = popoverOf(item)!;
  returnFocus.set(popover, returnTo);
  popover.showPopover({ source: item });
  const menu = popover.querySelector<HTMLElement>(':scope > [role="menu"]')!;
  if (focus === "menu") {
    menu.focus({ preventScroll: true });
  } else if (focus === "first-item") {
    menu.querySelector<HTMLElement>(':scope > [role="menuitem"]')?.focus({ preventScroll: true });
  }
}
