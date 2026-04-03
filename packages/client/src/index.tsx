/* @refresh reload */
import { render } from "solid-js/web";
import "./globals.css";
import App from "./App.tsx";

const root = document.getElementById("root");

if (!root) {
  throw new Error("Root element not found");
}

render(() => <App />, root);
