import { createContext, useContext, type ParentProps } from "solid-js";
import type { JSX } from "@solidjs/web";

const SUBMENU_DELAY_MS = 400;

type MenuParent = {
  inBar: boolean;
  closeAll: () => void;
  hoverItem: (item: HTMLButtonElement, open?: () => void) => void;
  unhoverItem?: () => void;
};

const MenuParentContext = createContext<MenuParent>();
const openers = new WeakMap<HTMLElement, (focusFirst?: boolean) => void>();

export function MenuBar(props: ParentProps) {
  // oxlint-disable-next-line no-unassigned-vars
  let bar!: HTMLDivElement;

  const openItem = () => bar.querySelector<HTMLButtonElement>(":scope > :has(+ :popover-open)");

  const context: MenuParent = {
    inBar: true,
    closeAll: () => bar.querySelector<HTMLElement>(":popover-open")?.hidePopover(),
    hoverItem: (item, open) => {
      const current = openItem();
      if (current && current !== item && open) {
        open();
      }
    },
  };

  return (
    <div
      ref={bar}
      role="menubar"
      onKeyDown={(event) => {
        if (event.key !== "ArrowLeft" && event.key !== "ArrowRight") {
          return;
        }
        const all = menuItems(bar);
        const open = openItem();
        const index = all.indexOf(open ?? (document.activeElement as HTMLButtonElement));
        if (index === -1) {
          return;
        }
        const delta = event.key === "ArrowLeft" ? -1 : 1;
        const next = all.at((index + delta) % all.length)!;
        const openNext = openers.get(next);
        if (open && openNext) {
          openNext(true);
        } else {
          next.focus();
        }
        event.preventDefault();
      }}
    >
      <MenuParentContext value={context}>{props.children}</MenuParentContext>
    </div>
  );
}

export function Menu(props: ParentProps<{ label: JSX.Element }>) {
  const parent = useContext(MenuParentContext)!;

  // oxlint-disable-next-line no-unassigned-vars
  let item!: HTMLButtonElement;
  let menu!: HTMLDivElement;

  let hovered: HTMLButtonElement | null = null;
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;

  const openSubmenu = () => menu.querySelector<HTMLElement>(":scope > :popover-open");

  const clearHover = () => {
    hovered = null;
    clearTimeout(hoverTimer);
  };

  const open = (focusFirstItem = false) => {
    parent.unhoverItem?.();
    const rect = item.getBoundingClientRect();
    menu.style.left = `calc(${parent.inBar ? rect.left : rect.right}px / var(--us, 1))`;
    menu.style.top = `calc(${parent.inBar ? rect.bottom : rect.top}px / var(--us, 1))`;
    if (!menu.matches(":popover-open")) {
      menu.showPopover({ source: item });
    }
    const target = focusFirstItem ? (menuItems(menu)[0] ?? menu) : menu;
    target.focus({ preventScroll: true });
  };

  const context: MenuParent = {
    inBar: false,
    closeAll: parent.closeAll,
    hoverItem: (target, openTarget) => {
      if (target === hovered) {
        return;
      }
      clearHover();
      hovered = target;
      target.focus({ preventScroll: true });
      hoverTimer = setTimeout(() => {
        if (openTarget) {
          openTarget();
        } else {
          openSubmenu()?.hidePopover();
        }
      }, SUBMENU_DELAY_MS);
    },
    unhoverItem: () => {
      clearHover();
      if (document.activeElement?.parentElement === menu) {
        (openSubmenu() ?? menu).focus({ preventScroll: true });
      }
    },
  };

  return (
    <>
      <button
        ref={(el) => {
          item = el;
          openers.set(el, open);
        }}
        role="menuitem"
        aria-haspopup="menu"
        onPointerDown={(event) => {
          if (event.button !== 0) {
            return;
          }
          event.preventDefault();
          if (parent.inBar && menu.matches(":popover-open")) {
            menu.hidePopover();
          } else {
            open();
          }
        }}
        onPointerMove={() => parent.hoverItem(item, open)}
        onPointerLeave={parent.unhoverItem}
        onClick={(event) => {
          event.preventDefault();
          if (event.detail === 0) {
            open(true);
          }
        }}
        onKeyDown={(event) => {
          if (event.key === (parent.inBar ? "ArrowDown" : "ArrowRight")) {
            open(true);
            event.preventDefault();
            event.stopPropagation();
          }
        }}
      >
        {props.label}
      </button>
      <div
        ref={(el) => {
          menu = el;
          item.popoverTargetElement = el;
        }}
        popover
        role="menu"
        tabindex={-1}
        onToggle={(event) => {
          if (event.newState === "open") {
            return;
          }
          clearHover();
          const focused = document.activeElement;
          if (!focused || focused === document.body || menu.contains(focused)) {
            item.focus({ preventScroll: true });
          }
        }}
        onKeyDown={(event) => {
          const all = menuItems(menu);
          const index = all.indexOf(document.activeElement as HTMLButtonElement);
          switch (event.key) {
            case "ArrowDown":
              all[(index + 1) % all.length]?.focus();
              break;
            case "ArrowUp":
              all.at(Math.max(index, 0) - 1)?.focus();
              break;
            case "ArrowLeft":
              if (parent.inBar) {
                return;
              }
              menu.hidePopover();
              break;
            default:
              return;
          }
          clearHover();
          event.preventDefault();
          event.stopPropagation();
        }}
      >
        <MenuParentContext value={context}>{props.children}</MenuParentContext>
      </div>
    </>
  );
}

export function MenuButton(props: ParentProps<{ onClick?: () => void }>) {
  const parent = useContext(MenuParentContext)!;

  const activate = () => {
    parent.closeAll();
    props.onClick?.();
  };

  return (
    <button
      role="menuitem"
      onPointerMove={(event) => parent.hoverItem(event.currentTarget)}
      onPointerLeave={parent.unhoverItem}
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
      {props.children}
    </button>
  );
}

function menuItems(container: Element) {
  return [...container.querySelectorAll<HTMLButtonElement>(':scope > [role="menuitem"]')];
}
