const TEXT_INPUT_TYPES = new Set([
	"",
	"text",
	"search",
	"url",
	"tel",
	"email",
	"password",
	"number",
]);

/** True for editable search/text fields where Cut / Paste / Undo should stay available. */
export function isEditableTextTarget(target: EventTarget | null): boolean {
	if (!(target instanceof Element)) return false;
	const el = target.closest("input, textarea, [contenteditable='true']");
	if (!el) return false;

	if (el instanceof HTMLInputElement) {
		if (el.disabled || el.readOnly) return false;
		return TEXT_INPUT_TYPES.has(el.type.toLowerCase());
	}
	if (el instanceof HTMLTextAreaElement) {
		return !el.disabled && !el.readOnly;
	}
	return (el as HTMLElement).isContentEditable;
}

/**
 * Block WebView right-click menus app-wide, except on editable text inputs
 * (native Cut / Paste / Undo / Copy). Ctrl+C is unaffected.
 */
export function installAppContextMenuGuard(): () => void {
	const onContextMenu = (event: MouseEvent) => {
		if (isEditableTextTarget(event.target)) return;
		event.preventDefault();
	};
	document.addEventListener("contextmenu", onContextMenu, true);
	return () => document.removeEventListener("contextmenu", onContextMenu, true);
}
