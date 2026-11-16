import { useEffect, useState, type ReactNode } from "react";

type ToastState = {
	id: number;
	message: string;
};

/**
 * Floating toast for transient tips (success / info).
 * Prefer this over sticky banners for one-shot feedback; keep banners for
 * persistent warnings and errors that need to stay visible.
 */
export function useToast() {
	const [toast, setToast] = useState<ToastState | null>(null);
	const [visible, setVisible] = useState(false);

	useEffect(() => {
		if (!toast) {
			setVisible(false);
			return;
		}
		setVisible(false);
		const showFrame = window.requestAnimationFrame(() => {
			setVisible(true);
		});
		const hideTimer = window.setTimeout(() => setVisible(false), 3000);
		const clearTimer = window.setTimeout(() => setToast(null), 3400);
		return () => {
			window.cancelAnimationFrame(showFrame);
			window.clearTimeout(hideTimer);
			window.clearTimeout(clearTimer);
		};
	}, [toast]);

	function showToast(message: string) {
		setToast({ id: Date.now(), message });
	}

	const toastNode: ReactNode = toast ? (
		<div
			className={`toast${visible ? " toast-visible" : ""}`}
			role="status"
			aria-live="polite"
		>
			{toast.message}
		</div>
	) : null;

	return { showToast, toastNode };
}
