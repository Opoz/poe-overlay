/* @refresh reload */
import { getCurrentWindow } from "@tauri-apps/api/window";
import { render } from "solid-js/web";
import GemsOverlay from "./gems/GemsOverlay";
import ActOverlay from "./guide/ActOverlay";
import GuideOverlay from "./guide/GuideOverlay";
import XpOverlay from "./guide/XpOverlay";
import LabOverlay from "./lab/LabOverlay";
import { followPanelOpacity } from "./look";
import MainWindow from "./main/MainWindow";
import TimerOverlay from "./timer/TimerOverlay";
import TreeBar from "./tree/TreeBar";
import TreeOverlay from "./tree/TreeOverlay";
import "@fontsource/barlow-semi-condensed/400.css";
import "@fontsource/barlow-semi-condensed/500.css";
import "@fontsource/barlow-semi-condensed/600.css";
import "@fontsource/barlow-semi-condensed/700.css";
import "./styles.css";

// One bundle serves every window; the window label picks the view.
const label = getCurrentWindow().label;
document.documentElement.dataset.window = label;
if (label !== "main") followPanelOpacity();

render(() => {
	if (label === "timer") return <TimerOverlay />;
	if (label === "tree") return <TreeOverlay />;
	if (label === "guide") return <GuideOverlay />;
	if (label === "act") return <ActOverlay />;
	if (label === "gems") return <GemsOverlay />;
	if (label === "lab") return <LabOverlay />;
	if (label === "treebar") return <TreeBar />;
	if (label === "xp") return <XpOverlay />;
	return <MainWindow />;
}, document.getElementById("root") as HTMLElement);
