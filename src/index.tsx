/* @refresh reload */
import { getCurrentWindow } from "@tauri-apps/api/window";
import { render } from "solid-js/web";
import MainWindow from "./main/MainWindow";
import TimerOverlay from "./timer/TimerOverlay";
import "./styles.css";

// One bundle serves every window; the window label picks the view.
const label = getCurrentWindow().label;
document.documentElement.dataset.window = label;

render(
	() => (label === "timer" ? <TimerOverlay /> : <MainWindow />),
	document.getElementById("root") as HTMLElement,
);
