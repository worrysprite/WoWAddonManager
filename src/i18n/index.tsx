import {
	createContext,
	useCallback,
	useContext,
	useMemo,
	useState,
	type ReactNode,
} from "react";
import {
	detectSystemLocale,
	resolveLocale,
	translate,
	type LocaleCode,
	type LocalePreference,
	type MessageKey,
} from "./messages";

type I18nContextValue = {
	preference: LocalePreference;
	locale: LocaleCode;
	systemLocale: LocaleCode;
	setPreference: (preference: LocalePreference) => void;
	setSystemLocale: (locale: string | null | undefined) => void;
	t: (key: MessageKey, vars?: Record<string, string | number>) => string;
};

const I18nContext = createContext<I18nContextValue | null>(null);

export function I18nProvider({ children }: { children: ReactNode }) {
	const [preference, setPreferenceState] = useState<LocalePreference>("system");
	const [systemLocale, setSystemLocaleState] = useState<LocaleCode>(() => detectSystemLocale());

	const locale = useMemo(
		() => resolveLocale(preference, systemLocale),
		[preference, systemLocale],
	);

	const setPreference = useCallback((next: LocalePreference) => {
		setPreferenceState(next);
	}, []);

	const setSystemLocale = useCallback((raw: string | null | undefined) => {
		setSystemLocaleState(raw ? resolveLocale(raw, raw) : detectSystemLocale());
	}, []);

	const t = useCallback(
		(key: MessageKey, vars?: Record<string, string | number>) => translate(locale, key, vars),
		[locale],
	);

	const value = useMemo(
		() => ({
			preference,
			locale,
			systemLocale,
			setPreference,
			setSystemLocale,
			t,
		}),
		[preference, locale, systemLocale, setPreference, setSystemLocale, t],
	);

	return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export function useI18n(): I18nContextValue {
	const ctx = useContext(I18nContext);
	if (!ctx) {
		throw new Error("useI18n must be used within I18nProvider");
	}
	return ctx;
}

export type { LocaleCode, LocalePreference, MessageKey };
export { detectSystemLocale, resolveLocale };