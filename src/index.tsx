/* @refresh reload */
import { getCurrentWindow } from "@tauri-apps/api/window";
import { render } from "solid-js/web";
import GemsOverlay from "./gems/GemsOverlay";
import ActOverlay from "./guide/ActOverlay";
import GuideOverlay from "./guide/GuideOverlay";
import LabOverlay from "./lab/LabOverlay";
import MainWindow from "./main/MainWindow";
import TimerOverlay from "./timer/TimerOverlay";
import TreeOverlay from "./tree/TreeOverlay";
import "./styles.css";

// One bundle serves every window; the window label picks the view.
const label = getCurrentWindow().label;
document.documentElement.dataset.window = label;

render(() => {
	if (label === "timer") return <TimerOverlay />;
	if (label === "tree") return <TreeOverlay />;
	if (label === "guide") return <GuideOverlay />;
	if (label === "act") return <ActOverlay />;
	if (label === "gems") return <GemsOverlay />;
	if (label === "lab") return <LabOverlay />;
	return <MainWindow />;
}, document.getElementById("root") as HTMLElement);
