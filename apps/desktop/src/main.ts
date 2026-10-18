import { mount } from "svelte";
import Start from "./Start.svelte";
import "./app.css";

// Start zeigt im Netzwerkbetrieb zuerst die Anmeldung, danach die Anwendung.
export default mount(Start, { target: document.getElementById("app")! });
