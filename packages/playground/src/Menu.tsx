import { createUniqueId, type JSX, type ParentProps } from "solid-js";

const SUBMENU_DELAY_MS = 400;

export function MenuBar(props: ParentProps) {
  // oxlint-disable-next-line no-unassigned-vars
  let bar!: HTMLDivElement;

  const openItem = () => menuItems(bar).find((item) => isOpen(popoverOf(item)));

  const moveInBar = (delta: 1 | -1) => {
    const all = menuItems(bar);
    const current = openItem() ?? (document.activeElement as HTMLButtonElement);
    const index = all.indexOf(current);
    if (index === -1) {
      return;
    }
    const next = all.at((index + delta) % all.length)!;
    if (openItem()) {
      openMenu(next, true);
    } else {
      next.focus();
    }
  };

  return (
    <div
      ref={bar}
      role="menubar"
      onPointerMove={(event) => {
        const item = (event.target as Element).closest<HTMLButtonElement>('[role="menuitem"]');
        const current = openItem();
        if (current && item?.parentElement === bar && item !== current && popoverOf(item)) {
          openMenu(item);
        }
      }}
      onKeyDown={(event) => {
        switch (event.key) {
          case "ArrowLeft":
            moveInBar(-1);
            break;
          case "ArrowRight":
            moveInBar(1);
            break;
          case "ArrowDown": {
            const item = event.target as HTMLButtonElement;
            if (item.parentElement !== bar || !popoverOf(item)) {
              return;
            }
            openMenu(item, true);
            break;
          }
          default:
            return;
        }
        event.preventDefault();
      }}
    >
      {props.children}
    </div>
  );
}

export function Menu(props: ParentProps<{ label: JSX.Element }>) {
  const id = createUniqueId();

  // oxlint-disable-next-line no-unassigned-vars
  let item!: HTMLButtonElement;
  // oxlint-disable-next-line no-unassigned-vars
  let menu!: HTMLDivElement;

  let hovered: HTMLButtonElement | null = null;
  let hoverTimer: ReturnType<typeof setTimeout> | undefined;

  const inBar = () => item.parentElement?.role === "menubar";
  const openSubmenu = () => menu.querySelector<HTMLElement>(":scope > :popover-open");

  const unhover = () => {
    hovered = null;
    clearTimeout(hoverTimer);
    if (document.activeElement?.parentElement === menu) {
      (openSubmenu() ?? menu).focus({ preventScroll: true });
    }
  };

  return (
    <>
      <button
        ref={item}
        role="menuitem"
        aria-haspopup="menu"
        popovertarget={id}
        onPointerDown={(event) => {
          if (!inBar() || event.button !== 0) {
            return;
          }
          event.preventDefault();
          if (isOpen(menu)) {
            menu.hidePopover();
          } else {
            openMenu(item);
          }
        }}
        onClick={(event) => {
          event.preventDefault();
          if (event.detail === 0) {
            openMenu(item, true);
          } else if (!inBar()) {
            openMenu(item);
          }
        }}
      >
        {props.label}
      </button>
      <div
        ref={menu}
        id={id}
        popover
        role="menu"
        tabindex={-1}
        onToggle={(event) => {
          if (event.newState === "open") {
            return;
          }
          clearTimeout(hoverTimer);
          hovered = null;
          const focused = document.activeElement;
          if (!focused || focused === document.body || menu.contains(focused)) {
            item.focus({ preventScroll: true });
          }
        }}
        onPointerMove={(event) => {
          const target = (event.target as Element).closest<HTMLButtonElement>('[role="menuitem"]');
          if (target?.parentElement !== menu) {
            unhover();
            return;
          }
          if (target === hovered) {
            return;
          }
          hovered = target;
          clearTimeout(hoverTimer);
          target.focus({ preventScroll: true });
          hoverTimer = setTimeout(() => {
            if (popoverOf(target)) {
              openMenu(target);
            } else {
              openSubmenu()?.hidePopover();
            }
          }, SUBMENU_DELAY_MS);
        }}
        onPointerLeave={unhover}
        onKeyDown={(event) => {
          const all = menuItems(menu);
          const focused = document.activeElement as HTMLButtonElement;
          const index = all.indexOf(focused);
          switch (event.key) {
            case "ArrowDown":
              all[(index + 1) % all.length]?.focus();
              break;
            case "ArrowUp":
              all.at(Math.max(index, 0) - 1)?.focus();
              break;
            case "ArrowRight":
              if (!popoverOf(focused)) {
                return;
              }
              openMenu(focused, true);
              break;
            case "ArrowLeft":
              if (inBar()) {
                return;
              }
              menu.hidePopover();
              break;
            default:
              return;
          }
          event.preventDefault();
          event.stopPropagation();
        }}
      >
        {props.children}
      </div>
    </>
  );
}

export function MenuButton(props: ParentProps<{ onClick?: () => void }>) {
  const activate = (item: HTMLElement) => {
    item.closest('[role="menubar"]')?.querySelector<HTMLElement>(":popover-open")?.hidePopover();
    props.onClick?.();
  };

  return (
    <button
      role="menuitem"
      onPointerUp={(event) => {
        if (event.button === 0) {
          activate(event.currentTarget);
        }
      }}
      onClick={(event) => {
        if (event.detail === 0) {
          activate(event.currentTarget);
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

function isOpen(popover: Element | null | undefined) {
  return popover?.matches(":popover-open") ?? false;
}

function popoverOf(item: HTMLButtonElement | null) {
  return (item?.popoverTargetElement ?? null) as HTMLElement | null;
}

function openMenu(item: HTMLButtonElement, focusFirstItem = false) {
  const menu = popoverOf(item);
  if (!menu) {
    item.focus();
    return;
  }
  const rect = item.getBoundingClientRect();
  const inBar = item.parentElement?.role === "menubar";
  menu.style.left = `calc(${inBar ? rect.left : rect.right}px / var(--us, 1))`;
  menu.style.top = `calc(${inBar ? rect.bottom : rect.top}px / var(--us, 1))`;
  if (!isOpen(menu)) {
    menu.showPopover({ source: item });
  }
  const target = focusFirstItem ? (menuItems(menu)[0] ?? menu) : menu;
  target.focus({ preventScroll: true });
}
