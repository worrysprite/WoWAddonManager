import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { installAppContextMenuGuard } from "./contextMenu";
import { I18nProvider } from "./i18n";

installAppContextMenuGuard();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
	<React.StrictMode>
		<I18nProvider>
			<App />
		</I18nProvider>
	</React.StrictMode>,
);
