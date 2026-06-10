import { mount } from "svelte";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import "./theme.css";
import App from "./App.svelte";
import AllTasks from "./lib/components/AllTasks.svelte";

const label = getCurrentWebviewWindow().label;
const Component = label === "all-tasks" ? AllTasks : App;

const app = mount(Component, {
  target: document.getElementById("app")!,
});

export default app;
